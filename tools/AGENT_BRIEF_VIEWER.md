# Brief for agents making Open BIM Suite show Rhino files as Rhino does

Viewer repo: C:\Users\joasg\GitHub\open-3d-viewer (GitHub OpenAEC-Foundation/open-bim-suite; Rust, iced
+ wgpu). The Rhino reader is the crate rhino-dll (C:\Users\joasg\GitHub\3dm-dll, pinned in the
viewer's Cargo.toml by git rev); the viewer's Rhino loader is src/loaders/rhino.rs (read it first:
Scene, draw(), props(), finish()). The model the loaders fill is src/model.rs (Layer: positions,
normals, colors, indices, lines, elements, props; ProjectData: info, sheets, dots, section, ...).
IFC export: src/export/ifc.rs.

## Rules
- Never work in C:\Users\joasg\GitHub\open-3d-viewer itself and never switch its branch (other
  sessions use it). Make your own worktree from the PR branch:
  `git -C C:\Users\joasg\GitHub\open-3d-viewer fetch origin` then
  `git -C C:\Users\joasg\GitHub\open-3d-viewer worktree add <scratchpad>\wt-<area> -b rhino-<area> origin/rhino-dots-fonts-georef`
  (scratchpad = C:\Users\joasg\AppData\Local\Temp\claude\1d795b5e1dedf9ba\d4b146b0-ee97-444d-825c-b1917862dca9\scratchpad).
  Commit on your branch; push only your branch (`git push -u origin rhino-<area>`); do NOT open a
  PR (the lead merges into PR #278). End commits with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- If the reader must change, do it in your own worktree of rhino-dll
  (`git -C C:\Users\joasg\GitHub\3dm-dll worktree add <scratchpad>\wt-dll-<area> -b <area> main`), commit
  there (do not push), and point the viewer's Cargo.toml at it with a `[patch]` or path only
  locally — tell the lead the crate branch so he merges and pins it.
- Builds: every cargo command through
  `powershell -NoProfile -ExecutionPolicy Bypass -File <scratchpad>\capped.ps1 -Command "<cargo cmd>" -Log <log> -WorkDir <worktree> -MemoryGB 6 -CpuPercent 15`
  with `-j 2`. The viewer is big: a test build takes ~10 minutes. Keep each tool call short (under 2
  minutes): run builds/tests in the background and wait for the notification; long calls stall you.
  Commit and push after each working step.
- Tests: `cargo test --lib -j 2 -- loaders::rhino export::` (plus your own). Test models made in
  Rhino 9 go in test-models/rhino with their make_*.py (see make_dots.py / make_layouts.py there and
  the crate's test-models/make_*.py): run Rhino 9 as
  `"C:\Program Files\Rhino 9\System\Rhino.exe" /nosplash /runscript="_-RunPythonScript <script> _-Exit"`
  with env MODELS_OUT; wrap main() in try/except writing *_error.txt; Rhino may not exit: wait ~90 s
  then stop ONLY the Rhino you started, by PID. Other agents use Rhino too.
- To see the app: build the desktop app from your worktree (`cargo build --release -j 2`, capped) and
  run it with a PRIVATE %APPDATA% (set APPDATA to a folder in your scratchpad before starting) so
  you do not take over other sessions' app; Rhino 9 can capture its own view
  (`RhinoDoc.ActiveDoc.Views.ActiveView.CaptureToBitmap`) for side-by-side comparison.
- Follow the existing style (short doc comments, no panics on bad input, limits).

## Report (under 350 words)
Branch(es) + head commit(s); what now shows as in Rhino and how it was checked; what not and why.
