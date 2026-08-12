# Codex Workflow

## Start prompt

Use `CODEX_START.txt` as the first prompt.

## Version discipline

1. Read `GOAL.md`, then the current version in `ROADMAP.md`.
2. Work only on approved current-version scope and its acceptance criteria.
3. Move roadmap work from planned to in progress and then completed only when
   implementation and applicable verification are complete.
4. Keep behavior documentation, `ISSUES.md`, `docs/ACCEPTANCE.md`, and
   `RELEASE.md` aligned with the result.
5. Run the checks required by `AGENTS.md` and report each acceptance criterion.
6. Create commits, tags, or releases only when the user explicitly requests
   them.

`ROADMAP.md` is forward-looking. Do not use it as a substitute for the release
history in `RELEASE.md` or the concrete defect register in `ISSUES.md`.

When the `$roadmap-git-workflow` skill is available, use it for adding,
promoting, re-scoping, completing, implementing, or auditing roadmap work so
these files remain aligned and verified milestones become scoped local commits.

## Audit prompt

```text
Audit the current repository against AGENTS.md, the active GOAL.md, architecture docs and acceptance criteria.
Do not add new features.
Find fake/stub production behavior, architecture violations, missing errors/tests, UI deviations, process leaks, persistence problems and unsafe process execution.
Record real defects in ISSUES.md, fix all defects related to the active goal, run the full relevant test suite and report exact results.
```
