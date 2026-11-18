# Architecture

```
threedm-cli ─┐
threedm-ffi ─┴─► threedm-read ─► threedm-model ─► threedm-core
                       └─────────────────────────────► threedm-core
```

## threedm-core: the archive

A 3dm file is a 32-byte header (`3D Geometry File Format`, then the archive version) and a
sequence of chunks. A chunk is a typecode and a value; a short chunk (high bit of the typecode)
holds only the value, a long one is followed by that many bytes of content, the last four a CRC
when the typecode has the CRC bit. Lengths are 4 bytes before archive version 50 and 8 bytes
from it. `Cur` is a cursor over the bytes of one chunk: little-endian numbers, strings
(UTF-16), uuids, colours, transforms, versioned chunks, compressed buffers, and `object()`
(`ReadObject`: class uuid, class data, user data). Every read checks the bytes left, so a
damaged file ends in an error, never in a panic or a huge allocation.

`write` is the opposite for tests: it writes the chunks, objects, meshes, layers and tables the
readers read, so tests can make files with exactly the case they test.

## threedm-model: what comes out

`File` holds the versions, the application that saved the file, the units (metres per unit),
the layers, materials and block definitions, the objects, and the records that could not be
read by table. Each `Object` has its `Attributes` (id, name, layer, material, colour and their
sources, visibility, mode and space), its user text and its `Geometry`: `Meshes` (with the
object type to show), `Lines` (polylines), `Points` (borrowed from the file, read when used),
`Instance` (block definition and transform), `Drawing` (lines and meshes together: dimensions
and leaders), `ClippingPlane` or `Skipped` (with the reason). `File.views` are the viewports of
its views, the active one first. A text, dimension or leader has its `Font` (of its first
character). `File.settings` holds the document settings (`Settings`: earth anchor point, model
base point and URL, construction planes, mesh, grid and render settings, current entries,
plug-ins); beside it the dimension styles with their fonts, the font table, the history
records, the plug-in tables as stored, the document user text, layer states and snapshots.

## threedm-read: from tables to geometry

`read` walks the tables (properties, settings, materials, layers, block definitions, dimension
styles, fonts, hatch patterns, objects, history records, user tables); a damaged record is
counted and skipped. Objects:

- meshes as stored (`ON_Mesh`, compressed or not);
- polysurfaces, surfaces and extrusions by the render meshes Rhino keeps with them, else by a
  mesh made from their exact geometry (`tessellate`): NURBS, revolved, sum and plane surfaces
  and the curves on them are evaluated; each face is cut by its trims and triangulated
  (constrained Delaunay), the edges two faces share are sampled once so that closed solids
  stay watertight, untrimmed faces take a structured grid, Rhino 2 polysurfaces are read in
  their own layout;
- Rhino 1 files (`v1`, archive version 1): no tables but top-level chunks: layers, the unit
  system and objects, each its geometry with its attributes (layer by index, colour, name):
  compressed meshes, points (with their arrows and text dots), curves of NURBS segments and
  faces and shells of trimmed NURBS surfaces, meshed as Breps are;
- SubD objects by their limit surface (`subd`), Catmull–Clark with Rhino's sector coefficients
  and creases;
- curves as polylines within Rhino's display tolerance;
- texts and hatches (`drafting`): texts set in a font at their dimension style's text height
  times its model space scale, rich text reduced to plain text; hatches filled, or drawn in their
  pattern's lines cut to their loops and dashed;
- dimensions, centre marks and leaders (`dimensions`), from the points openNURBS stores in the
  annotation's plane and their dimension style (read in full: sizes, formats, arrowheads, landing,
  text location, length display): extension lines, dimension lines and arcs, arrowheads, the
  value set in the font; one `Geometry::Drawing` of lines and meshes per object;
- point clouds, points and block instances as they are;
- clipping planes (`ON_ClippingPlaneSurface`): the rectangle of their plane surface, the plane
  (its normal points to the side cut away), enabled, the viewports they clip, their depth and
  the objects and layers they clip (Rhino 8's selective clipping), and the viewports of the
  file's views (`TCODE_SETTINGS_VIEW_LIST`: id, name, perspective; the active one first) to match
  them with.

Beside the objects:

- the settings (`settings`): every chunk `ON_3dmSettings::Read` handles besides the units,
  views and annotation settings: the plug-in list, render and analysis mesh settings, named
  construction planes (and those of the view list's views, by viewport id), the current
  layer, material, colour and wire density, render settings (Rhino 5's layout and Rhino 6's),
  grid defaults, the model URL and the settings attributes (linetype scale, current plot colour
  and linetype, active view, model base point, earth anchor point, IO settings, custom render
  mesh settings, current entries by id); the history record table (`ON_HistoryRecord` and its
  `ON_Value`s; geometry values kept as bytes); the user tables (`TCODE_USER_TABLE`) as stored,
  and of them the document user text (`ON_DocumentUserStringList`), the named layer states
  (Rhino Bonus Tools' table: each state's layers as `ON_Layer`) and the snapshots (the
  Snapshots plug-in's XML, by name);
- fonts (`fonts`): `ON_Font` of the dimension styles, the font table of Rhino 4 and 5
  (`ON_TextStyle`), and the font of a text, dimension or leader: its rich text's font table and
  the `\f`, `\b` and `\i` in force at its first character, else its own style's or its
  dimension style's font.

Limits: at most `MAX_VERTICES` mesh vertices are read from a file and at most 120 000 SubD faces
are made per object; what goes beyond is left out with the reason.

## threedm-ffi and threedm-cli

`Document` keeps the bytes of a file and the model read from them together. The C ABI opens a
document and answers JSON requests on it (`query` is also the Rust entry point); the command
line tool prints those answers.
