# rhino-dll

Rust library (and DLL) that reads Rhino `.3dm` files without Rhino, for use by
[Open BIM Suite](https://github.com/OpenAEC-Foundation/open-bim-suite) and other tools.

**Status:** private, alpha. Reads every Rhino version from Rhino 1 to Rhino 9.

## Scope

- Input: `.3dm` files of Rhino 1 to 9 (archive versions 1, 2, 3, 4, 5, 50, 60, 70, 80 and 90).
- Output: a neutral model (`threedm-model`): units, layers, materials, block definitions and
  objects with their attributes and user text, each object as meshes, polylines, points or a
  block instance. Exact geometry is meshed as Rhino meshes it on opening a file, so that files
  saved without render meshes (by a script, another program, or with "save small") show whole:
  polysurfaces (Breps, Rhino 2's layout included), surfaces, extrusions (mitered ends too) and
  SubD objects (their limit surface). Curves become polylines, texts are set in a font at their
  dimension style's height and model space scale, hatches are filled or drawn in their pattern's
  lines (with Rhino 8's background fill behind them and their boundary when the hatch has them).
  Linetypes come with Rhino 8's width (and its unit), cap, join and taper; each curve has its
  `stroke` (that width, also in model units, cap, join, taper and its print width and colour) so
  that a program can draw wide lines. Section styles (Rhino 9's table, and the own styles of
  layers and objects of Rhino 8) and where layers, objects and clipping planes take them from. Dimensions (linear, aligned, angular, radius, diameter, ordinate), centre marks and
  leaders are drawn as Rhino draws them: extension lines, dimension lines and arcs, arrowheads of
  the style's kind and the value formatted as the style says (lines, and meshes for arrowheads
  and text). Clipping planes come with their rectangle, plane, depth, the objects they clip
  (selective clipping) and the viewports they clip, and the file's views with them.
- A C ABI (`threedm.dll`) answering JSON requests, and a command line tool (`threedm`).
- Drawing the model (colours by layer, object, parent or material, instance placement, hidden
  layers) is up to the program that uses it; Open BIM Suite's Rhino loader is an example.

## Crates

| Crate | What it does |
|---|---|
| `threedm-core` | The openNURBS archive: chunks with 4- and 8-byte lengths, versioned and anonymous chunks, compressed buffers, `ReadObject` with user data, user strings. Also a small 3dm writer for tests. |
| `threedm-model` | The neutral model: `File`, `Layer`, `Material`, `BlockDefinition`, `Object`, `Attributes`, `Mesh`, `Geometry`. |
| `threedm-read` | The readers: tables and objects (`read`, `read_with`, `Document`), meshing of exact geometry (`tessellate`), SubD (`subd`), texts and hatches (`drafting`), dimensions and leaders (`dimensions`). |
| `threedm-ffi` | The C ABI, built as `threedm.dll` / `libthreedm.so`. |
| `threedm-cli` | The `threedm` command line tool. |

Crate names cannot start with a digit, hence `threedm-`.

## Use

From Rust, as a git dependency:

```toml
threedm-read = { git = "https://github.com/OpenAEC-Foundation/rhino-dll", rev = "…" }
```

```rust
let data = std::fs::read("model.3dm")?;
let file = threedm_read::read(&data)?;
for object in &file.objects {
    if let threedm_read::Geometry::Meshes(meshes, kind) = &object.geometry {
        println!("{} {kind}: {} meshes", object.attributes.name, meshes.len());
    }
}
```

`read` sets texts in the built-in Inter (feature `font`, on by default); `read_with` takes the
font to use, or none. Meshes are read up to `MAX_VERTICES` vertices in all, so that a damaged or
hostile file cannot exhaust memory; what does not fit is left out and says why.

From C:

```c
ThreedmDocument *doc = threedm_open("model.3dm");
char *info = threedm_query(doc, "{\"query\": \"info\"}");
threedm_free(info);
threedm_close(doc);
```

Requests: `info`, `layers`, `materials`, `blocks`, `objects` (with `limit` and `offset`) and
`object` (by `id`, with its geometry); see `crates/threedm-ffi/src/lib.rs`.

From the command line:

```
threedm info model.3dm
threedm objects model.3dm 10
threedm object model.3dm d8c5f31f-a751-4c64-8c36-5d02a72af021
```

## Build and test

```
cargo build --release
cargo test --release --workspace
```

The tests read the models in `test-models` and compare the meshed areas of their objects, the
size of their texts and the lines and texts of their dimensions and leaders with what Rhino 9
measures and draws.

## How it was made

The readers are written from McNeel's openNURBS source, which McNeel publishes as the reference
for reading and writing 3dm files. Nothing of openNURBS is compiled in or copied. Rhino 9 was
used as an oracle: test models made by the scripts in `test-models` (run in Rhino with
`_-RunPythonScript`), and the areas and outlines Rhino reports for them.

## Test data

`test-models` holds only models we made ourselves, with the scripts that make them. Do not
commit models from others (McNeel's example files included) or exports derived from them.

## Licence

LGPL-3.0-or-later, like Open BIM Suite, where this code started. The built-in font is Inter
(SIL Open Font License, `crates/threedm-read/fonts/Inter-LICENSE.txt`).

Rhinoceros and openNURBS are trademarks of Robert McNeel & Associates. This project is not
affiliated with or endorsed by McNeel.
