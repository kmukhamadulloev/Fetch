# Codex Workflow

## Start prompt

Use `CODEX_START.txt` as the first prompt.

## Phase discipline
Do not ask Codex to implement every phase in one giant run.

When current acceptance criteria pass:
1. point `GOAL.md` to the next phase file;
2. commit the completed phase;
3. start a new Codex task.

## Audit prompt

```text
Audit the current repository against AGENTS.md, the active GOAL.md, architecture docs and acceptance criteria.
Do not add new features.
Find fake/stub production behavior, architecture violations, missing errors/tests, UI deviations, process leaks, persistence problems and unsafe process execution.
Record real defects in ISSUES.md, fix all defects related to the active goal, run the full relevant test suite and report exact results.
```
