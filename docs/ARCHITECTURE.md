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
and leaders) or `Skipped` (with the reason).

## threedm-read: from tables to geometry

`read` walks the tables (properties, settings, materials, layers, block definitions, dimension
styles, hatch patterns, objects); a damaged record is counted and skipped. Objects:

- meshes as stored (`ON_Mesh`, compressed or not);
- polysurfaces, surfaces and extrusions by the render meshes Rhino keeps with them, else by a
  mesh made from their exact geometry (`tessellate`): NURBS, revolved, sum and plane surfaces
  and the curves on them are evaluated; each face is cut by its trims and triangulated
  (constrained Delaunay), the edges two faces share are sampled once so that closed solids
  stay watertight, untrimmed faces take a structured grid, Rhino 2 polysurfaces are read in
  their own layout;
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
- point clouds, points and block instances as they are.

Limits: at most `MAX_VERTICES` mesh vertices are read from a file and at most 120 000 SubD faces
are made per object; what goes beyond is left out with the reason.

## threedm-ffi and threedm-cli

`Document` keeps the bytes of a file and the model read from them together. The C ABI opens a
document and answers JSON requests on it (`query` is also the Rust entry point); the command
line tool prints those answers.
