# Refresh Trellis Bootstrap Guidelines After OpenSpec Conversion

## Goal
Make the converted Trellis maintenance layer accurately describe the current repository state so future work can continue from Trellis-first docs without bootstrap confusion or missing provenance guidance.

## Requirements
- Update Trellis workflow and guide docs that still describe the repository as having only a minimal bootstrap skeleton when the repo already contains local Trellis scripts and Codex/Trellis support files.
- Clarify repository hygiene around generated conversion workspace content versus committed Trellis maintenance artifacts, without performing the old-framework cleanup in this task.
- Fix or document the most visible provenance/metadata gaps discovered during the conversion review so archived task context does not overstate what `source-manifest.yaml` currently preserves.
- Keep `.trellis/spec/` as the active source of truth and preserve `.trellis/tasks/archive/` plus `.trellis/legacy/specs/` as provenance.

## Acceptance Criteria
- [ ] `.trellis/workflow.md` and relevant guide docs describe the current bootstrap/runtime state consistently.
- [ ] Trellis docs explain what new work should edit, what historical imports are for, and what conversion workspace directories are not part of the active maintenance layer.
- [ ] Any archive/provenance guidance added in this task does not claim metadata that is not actually present in converted files.
- [ ] Validation relevant to this docs/bootstrap task passes, and the final report names any remaining non-blocking conversion issues.

## Technical Notes
- Scope is limited to Trellis/bootstrap guidance and repository hygiene documentation.
- The user will manually clean up old-framework content later; do not delete legacy OpenSpec artifacts in this task.
- Prefer correcting inaccurate guidance over inventing new workflow layers.
