# Brief for agents extending rhino-dll (Rust reader of Rhino .3dm files)

Repo: C:\Users\joasg\GitHub\3dm-dll (remote OpenAEC-Foundation/rhino-dll, branch main). Crates:
threedm-core (archive: chunks, objects, user data; `write` = small 3dm writer for tests),
threedm-model (neutral model types), threedm-read (readers; lib.rs reads the tables and objects,
read_geometry dispatches object classes), threedm-ffi (JSON queries for `threedm` CLI / dll),
threedm-cli. Read README.md and docs/ARCHITECTURE.md first.

## Rules
- Do NOT work in C:\Users\joasg\GitHub\3dm-dll itself and never switch its branch. Make your own
  worktree: `git -C C:\Users\joasg\GitHub\3dm-dll worktree add <scratchpad>\wt-<area> -b <area> main`
  (scratchpad = C:\Users\joasg\AppData\Local\Temp\claude\1d795b5e1dedf9ba\d4b146b0-ee97-444d-825c-b1917862dca9\scratchpad).
  Commit on your branch; do NOT push (the lead merges). End commit messages with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Builds: every cargo command through
  `powershell -NoProfile -ExecutionPolicy Bypass -File <scratchpad>\capped.ps1 -Command "<cargo cmd>" -Log <log> -WorkDir <your worktree> -MemoryGB 3 -CpuPercent 15`
  with `-j 1` (several agents build at once; the machine is capped). Keep each tool call short and
  run long commands in the background so your stream does not stall.
- Clean-room: write from McNeel's openNURBS source, the published reference for reading and
  writing 3dm files; local checkout:
  C:\Users\joasg\AppData\Local\Temp\claude\a318945cf73b27fc\d4b146b0-ee97-444d-825c-b1917862dca9\scratchpad\ref\opennurbs
  (its example_files hold McNeel's sample files V1..V8; never commit them).
- Oracle: Rhino 9 (`C:\Program Files\Rhino 9\System\Rhino.exe`). Pattern: a generator script in
  test-models/make_<area>.py (see test-models/make_layouts.py, make_attributes.py) run as
  `"C:\Program Files\Rhino 9\System\Rhino.exe" /nosplash /runscript="_-RunPythonScript <script> _-Exit"`
  with env MODELS_OUT; wrap main() in try/except writing <name>_error.txt; Rhino may not exit by
  itself: wait ~90 s, then stop ONLY the Rhino process you started, by its PID. Other agents use
  Rhino at the same time: start your own instance, never touch others. The script builds the test
  model, saves <name>.3dm and writes <name>.json with what Rhino reads back (File3dm.Read / RhinoCommon).
  Commit the .3dm and .json made by Rhino 9 into test-models (they are ours), with a test in
  crates/threedm-read/tests/models.rs comparing every field.
- Keep new code in new modules where possible; touch lib.rs only for the table/settings dispatch
  and a `mod` line, so merges with the other agents stay small. Follow the existing style (short
  doc comments saying what is read, constants named after openNURBS TCODEs, `err(...)`, no panics
  on bad input, limits on counts and sizes).
- Expose new data in threedm-model (plain types), in the FFI `info`/`object` JSON, and document it
  in docs/ARCHITECTURE.md (your lines) and README.md scope.
- `cargo test --workspace --release -j 1` green before your final commit.

## Report (under 350 words)
Branch + head commit; what is read and how it was checked against Rhino 9 (match per field); what
is not read and why.
