# Coding workflow

**Kicks in when:** the task at hand is source-code changes — new/modified
functions, a script beyond a thin executor, anything with a test suite to
extend. Applies per-task, not per-project — a `docs-driven` project still
uses this workflow for the parts of a task that are actual code.

For projects primarily driven by writing and changing code.

## Default toolchain

Spec-first flow before code: `/grill-with-docs` → implement with `/tdd`. Use
`/to-spec`/`/to-tickets` instead when the work is already broken into a plan
or slices — that's the exception, not the default first step. Skip the spec
step only if explicitly told to for that request — bugs included (see below).

**Every bug lands in a spec — target: 100% spec coverage of every app.**
Whenever a bug is found (while fixing something else, in review, from a user
report, during release prep), before fixing it:

1. **Look for the covering spec/PRD**: search the project's specs for the
   affected module, route, setting or UI surface.
2. **Covered → find the overlap and update that spec.** Name the requirement,
   acceptance scenario or edge case the bug violates — or the one the spec
   never stated (a spec that only says a setting is *exposed*, never what it
   *does*, is the classic gap). Correct or add it, and record the fix as a
   task that links the issue.
3. **Not covered → write a new spec** for the whole surface the bug lives in,
   not just the bug, so the gap closes for good.
4. **Escape analysis (shift left)**: add a row to the coverage ledger's
   Escapes table: where the bug was found, and the lowest layer that
   *should* have caught it (`spec` / unit / integration / api / e2e). Put
   the regression test at that layer, tagged with the requirement it proves.
5. File the issue, fix with `/tdd` where there's a test suite to extend, and
   cross-link spec ↔ issue ↔ PR.

The one exception is a literal config-only change with no behavior (a
version bump, an env var value): it skips the test and the spec, but not the
review that motivated it.

## Spec-kit toolchain (opt-in alternative)

If this project has adopted [spec-kit](https://github.com/github/spec-kit)
(`.specify/` exists, features tracked as `specs/NNN-<name>/{spec,plan,tasks}.md`)
— use its own commands in place of the default flow above, not alongside it:
optionally `/grill-with-docs` first for open-ended clarification (it's still
useful for questions spec-kit's own commands won't surface on their own), then
`/speckit-specify` → `/speckit-plan` → `/speckit-tasks` → implement each task
(same `/tdd` rule as above — TDD where the codebase has a test suite to
extend) → `/speckit-converge` before calling the feature done. Not installed
by default — a project opts in by running `specify init --here` once (adds
`.specify/`, the `speckit-*` skills, and the `specs/` convention), then notes
the choice in its own `CLAUDE.md` (this file stays the shared default; the
project's own file is where "we use spec-kit" gets recorded as a living,
project-owned decision — see `MIGRATE.md`'s living-docs rule).

Bugs follow the default toolchain's "every bug lands in a spec" rule above,
in spec-kit terms: grep `specs/*/spec.md` for the covering spec; if found,
correct/add the violated requirement in its `spec.md` and append a bugfix
phase to its `tasks.md` linking the issue (tick it when the fix lands); if
none, run `/speckit-specify` for the surface. A one-line fix still updates
the spec that covers it.

**Release Checklist, spec-kit projects**: step 4a (spec/PRD coverage) means
running `/speckit-converge` against every spec merged since the last release
tag, not just the one currently shipping — see `CLAUDE.md`'s Release
Checklist.

**Coverage audits and retroactive specs**: for projects that keep one
spec/PRD per feature, periodically check that every shipped module/route/
surface actually traces back to one — not just that the current task's spec
matches its own code (that only catches drift within one feature, not gaps
between them). When a gap turns up — shipped functionality nobody wrote a
spec for — don't leave it undocumented forever and don't retroactively
invent a fake "plan" for work already done; write a lightweight *retroactive*
record instead: same format as a normal spec/PRD, marked as already
implemented, describing the feature as it actually behaves today (verified
by reading the code, not guessing), with a short note on why it's
retroactive and what to do differently going forward. Spec-kit projects: this
is a spec.md with `Status: Implemented (retroactive spec — see Notes)` and
no plan.md/tasks.md, since there's nothing left to build.

**Spec coverage — target 100%, traced to tests** (when `spec-coverage` is enabled in `.claude/profile.yaml` — on by default): see
`docs/workflows/spec-coverage.md`. In short:
- `/spec-audit` (`.claude/skills/spec-audit`) scans for unspecced surfaces and untested
  requirements, and rebuilds the ledger (`specs/COVERAGE.md`).
- Every requirement states its observable effect and a `*Verify: <layer>*` line.
- Tests carry a `spec: NNN/FR-XXX` tag.
- **Tripwire, in every phase**: unspecced behavior noticed while doing something else gets
  logged (ledger row, gap issue, "Unspecced found" in the end-of-task report). Unspecced
  behavior about to change gets specced first.
- Spec-kit projects: record this as a constitution principle so `/speckit-*` commands enforce
  it.

