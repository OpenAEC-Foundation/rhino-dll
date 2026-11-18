# Tools for working on rhino-dll

- `rhino_silent.ps1` — runs a Rhino 9 Python script (the test-model generators `test-models/make_*.py`)
  on a separate, invisible Windows desktop: no Rhino window, splash or dialog can appear on the
  screen or take the focus. `rhino_silent.ps1 -Script test-models\make_x.py -Out <folder>`.
  Rhino must never be started visibly (not with `/runscript` on the normal desktop).
  (`-McpPort` tries to start the rhinomcp bridge there; that bridge does not start on an invisible
  desktop, so it is not usable yet.)
- `capped.ps1` — runs a build in a Windows job object capped at a memory size and CPU share:
  `capped.ps1 -Command "cargo test --workspace --release -j 2" -Log build.log -WorkDir . -MemoryGB 8 -CpuPercent 25`.
- `AGENT_BRIEF_RHINO_DLL.md`, `AGENT_BRIEF_VIEWER.md` — the briefs given to agents working on the
  reader and on the viewer (paths in them are of the machine they were written on). The rule that
  Rhino may only run silently applies to every agent.
