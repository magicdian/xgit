# Repository and Conversion State

> Grounded repository snapshot for converted Trellis maintenance.

## Snapshot

- Source framework: openspec
- Target framework: trellis
- Primary implementation root: `xgit/`
- Primary language: Rust
- Preprocess entities: 25
- Exported relations: 41
- Enhanced analysis coverage: 25/25
- Deterministic apply output count: 25
- Local Trellis scripts present: yes
- Project-local agent support present: `.agents/`, `.codex/`

## Trellis Layout State

- Legacy specs copied to `.trellis/legacy/specs/`: 9
- Active task directories under `.trellis/tasks/`: 0
- Archived task directories under `.trellis/tasks/archive/`: 16
- Task context files present: 16/16

## Grounded Spec Outputs

- `.trellis/spec/backend/rust-cli-runtime.md`
- `.trellis/spec/frontend/terminal-setup-ui.md`

## Runtime Contract Notes

- Workflow bootstrap file should live at `.trellis/workflow.md`.
- Backend / frontend / guide indexes should exist before Trellis skills run.
- After conversion, grounded Trellis docs under `.trellis/spec/` and new work under `.trellis/tasks/` are the active maintenance layer.
- Historical source imports should remain in `.trellis/tasks/archive/`, not the active task pool.
- `.trellis/legacy/specs/` and `.trellis/tasks/archive/` are provenance artifacts for traceability and recovery, not the default authoring surface for routine development.
- The current converted workspace already contains local `.trellis/scripts/` helpers and project-local agent support; `trellis update` and `trellis init` are optional refresh steps, not a prerequisite for normal work here.

## Bootstrap Stages

1. **Post-conversion baseline**
   - `.trellis/workflow.md`, grounded docs under `.trellis/spec/`, imported history under `.trellis/tasks/archive/`, portable task context files, and local `.trellis/scripts/` helpers should already exist in the current converted workspace.
2. **Optional runtime refresh**
   - Run `trellis update` and `trellis init` only when you intentionally want to refresh or standardize upstream tooling, then re-check the grounded docs and task contexts against the repo state.
3. **IDE-specific bootstrap**
   - `.codex/` may already be an active project-local support directory; `.codex_bak/` is backup or provenance output rather than active workflow state.

## Provenance Notes

- Imported task directories preserve normalized `prd.md`, `design.md`, `source-tasks.md`, and `task.json` metadata for Trellis-first continuation.
- `task.json` is the richest imported task metadata source because it records `sourcePath`, converted status, preserved-source file names, and any extracted summaries the converter managed to keep.
- `source-manifest.yaml` may be only a lightweight conversion marker for some imported tasks; do not assume it contains full lifecycle, acceptance, or estimate data unless the file explicitly does so.
- `.transpec/` is the converter workspace and `.codex_bak/` is backup output; neither directory is part of the active Trellis maintenance layer for routine feature work.

## Maintenance Expectations

1. Start new work from grounded Trellis docs and create new tasks under `.trellis/tasks/`.
2. Keep `.trellis/spec/` aligned with the repository's current runtime and interaction-surface ownership.
3. Treat `.trellis/legacy/specs/` and `.trellis/tasks/archive/` as read-only provenance unless you are intentionally re-importing history.
4. For imported task provenance, read `task.json`, `design.md`, and `source-tasks.md` before assuming `source-manifest.yaml` has the full story.
5. If your team regenerates imported history or grounded guides, review `.trellis/workflow.md`, the spec indexes, and task context files before continuing development.
