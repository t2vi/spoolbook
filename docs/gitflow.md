# Git Workflow

Gitflow (Vincent Driessen model). Two long-lived branches, five short-lived branch types.

## Branches

- **`main`** — production. Every commit on `main` is a tagged release. Protected: no direct pushes.
- **`develop`** — integration. Default PR target for feature work (`gh pr create --base develop`). Always deployable to staging. Keep GitHub's *default branch* setting on `main` if the repo is a GitHub template — "Use this template" copies only the default branch, so new repos should start from the released state, not unreleased `develop` work.
- **`feature/<name>`** — a new capability. Branches from `develop`, merges back into `develop` via PR. Delete after merge.
- **`bugfix/<name>`** — a fix for something already on `develop`/`main`. Same flow as `feature/*`.
- **`chore/<name>`** — maintenance, infra, CI, deps — no user-facing change. Same flow as `feature/*`.
- **`release/vX.X.X`** — release stabilization *and* release prep. Branches from `develop` once it's feature-complete for the release. Only bugfixes land here — no new features — plus the version bump, changelog, and release notes themselves; that prep step never happens inline on a `feature/*`/`bugfix/*`/`chore/*` branch, even for a release that's really just shipping one already-finished fix. Merges into **both** `main` and `develop`, tagged on `main`.
- **`hotfix/<name>`** — urgent production fix. Branches from `main`. Merges into **both** `main` and `develop`, tagged on `main`.

Always branch out before starting work — never keep editing on `main`/`develop`, and don't
reuse a branch left over from a different, unrelated prior task; cut a fresh one named for the
task actually at hand.

## Feature flow

```bash
git checkout develop && git pull
git checkout -b feature/<name>   # or bugfix/<name>, chore/<name> — same flow
# ...work, commit...
git push -u origin feature/<name>
gh pr create --base develop
# after merge:
git branch -d feature/<name>
```

## Cutting a release

```bash
git checkout develop && git pull --ff-only
git checkout -b release/vX.X.X
# bugfixes only, plus the version bump / CHANGELOG / release notes for this release —
# see the Release Checklist in CLAUDE.md
```

Then ship it in two steps. Each runs in a `( set -e; ... )` subshell so it stops at the first
failure — a plain `set -e` pasted into an interactive shell would close it instead. Don't run
them as one paste: a merge attempted before `main-merge-guard` finishes is refused, and without
the stop everything after it (tag, release) lands on the old `main`. Don't append `&&`/`||` to a block either — shells
ignore `set -e` inside a command whose exit status is being tested.

```bash
# 1. open the PR into main (only accepts PRs — see "Merge guard")
( set -e
  git push -u origin release/vX.X.X
  gh pr create --base main --head release/vX.X.X --title "Release vX.X.X" --fill
)

# 2. once the PR shows its checks: wait for green, merge, tag, publish, merge back
( set -e
  gh pr checks release/vX.X.X --required --watch
  gh pr merge release/vX.X.X --merge
  git checkout main
  git pull --ff-only
  git tag -a vX.X.X -m "vX.X.X"
  git push origin vX.X.X
  gh release create vX.X.X --verify-tag --notes-file docs/releases/vX.X.X.md
  git checkout develop
  git pull --ff-only
  git merge --no-ff --no-edit release/vX.X.X
  git push origin develop
  git branch -d release/vX.X.X
  git push origin --delete release/vX.X.X 2>/dev/null || true   # already gone if delete-branch-on-merge is on
)
```

If CI publishes release artifacts on tag push or GitHub Release creation, pushing the tag (or running `gh release create vX.X.X`) triggers it — see the project's `.github/workflows/`.

## Hotfix

```bash
git checkout main && git pull --ff-only
git checkout -b hotfix/<name>
# ...fix, commit...
```

Same two-step ship as a release:

```bash
# 1. open the PR into main
( set -e
  git push -u origin hotfix/<name>
  gh pr create --base main --head hotfix/<name> --fill
)

# 2. once the PR shows its checks: wait for green, merge, tag, merge back
( set -e
  gh pr checks hotfix/<name> --required --watch
  gh pr merge hotfix/<name> --merge
  git checkout main
  git pull --ff-only
  git tag -a vX.Y.Z -m "vX.Y.Z"   # patch bump, e.g. v1.4.2 → v1.4.3
  git push origin vX.Y.Z
  git checkout develop
  git pull --ff-only
  git merge --no-ff --no-edit hotfix/<name>
  git push origin develop
  git branch -d hotfix/<name>
  git push origin --delete hotfix/<name> 2>/dev/null || true   # already gone if delete-branch-on-merge is on
)
```

## Merge guard

Only `release/*` and `hotfix/*` may merge into `main` — never `develop` or a topic branch
directly. Two layers:

1. `.github/workflows/main-merge-guard.yml` — fails any PR into `main` whose source branch isn't
   `release/*`/`hotfix/*`.
2. Branch protection on `main` (one-time, per repo, needs admin) — makes that check required,
   requires a PR for every change (no direct pushes), and applies to admins too.

`scripts/repo-setup.sh` does all of this idempotently — creates `develop` if missing, turns on
delete-branch-on-merge, applies the protection below once the workflow is on `main` —
`--check` reports without changing anything. `MIGRATE.md` runs it on every migration. The raw
call, for reference:

```bash
gh api -X PUT "repos/{owner}/{repo}/branches/main/protection" --input - <<'JSON'
{
  "required_status_checks": { "strict": false, "contexts": ["main-merge-guard"] },
  "enforce_admins": true,
  "required_pull_request_reviews": { "required_approving_review_count": 0 },
  "restrictions": null
}
JSON
```

Without step 2 the check only shows red — the merge button still works. Private repos need a
paid GitHub plan for branch protection.

## Claude Code sessions

All sessions — interactive and background alike — edit in place on the checked-out branch; `.claude/settings.json` sets `worktree.bgIsolation: none` so background sessions don't isolate either. Use a worktree (`EnterWorktree`) only when explicitly asked.

## Recovering from an amend after push

If a commit already pushed to a remote branch gets `--amend`ed locally, the next `push` is
rejected (non-fast-forward) — the local and remote copies of that commit have diverged. Don't
force-push to "fix" it unless the amend intentionally needs to overwrite what's public; usually
the amend just folded in one more change that should've been its own commit. Undo it instead —
no force-push needed:

```bash
git log --oneline -1 origin/<branch>   # the commit that's actually public
git reset --soft <that-commit-sha>     # rewinds HEAD to it; working tree/index untouched
git commit -m "the extra change"       # what got folded in via amend, now its own commit
git push                                # fast-forwards cleanly — no --force
```
