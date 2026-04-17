# Pre-Implementation Checklist

> Grounded Trellis-first checklist for continuing work in a converted repository.

## Before You Touch Code

- Start from `.trellis/spec/` and `.trellis/tasks/`, not from preserved legacy artifacts, unless you explicitly need provenance.
- Read the task's local `prd.md`, `task.json`, and any preserved `design.md` or `source-tasks.md` before implementing.
- Read the grounded backend or frontend guide that matches the runtime or interaction surface you are about to change.

## Boundary Checks

- If the task changes both runtime code and terminal/UI behavior, read [Cross Layer Thinking Guide](./cross-layer-thinking-guide.md) before editing.
- If the task adds new helpers, guide paths, config keys, or repeated logic, read [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md) before editing.
- If the task came from imported history, skim [Repository And Conversion State](./repository-and-conversion-state.md) so archive, legacy-spec, and regeneration rules stay aligned.

## Trellis-First Rules

- Treat `.trellis/spec/` as the current source of truth for ongoing development.
- Treat `.trellis/legacy/specs/` and `.trellis/tasks/archive/` as provenance that can explain why something exists, not as the default place to continue work.
- Treat `.transpec/` and `.codex_bak/` as conversion workspace or backup directories unless the task is explicitly about the converter itself.
- For imported tasks, prefer `task.json`, `design.md`, and `source-tasks.md` for detailed provenance; `source-manifest.yaml` can be only a lightweight conversion marker.
- When you discover source wording that still talks about the pre-conversion framework, carry the intent forward into Trellis docs or task context instead of reintroducing the old workflow as the active one.
