# BOARD.md Format

`BOARD.md` is the persistent, human-readable project state. It lives at the
root of the project being experimented on (not in this skills repo). It is
both machine-parsed and human-edited: Rust owns the machine-controlled
fields, humans and agents own the prose sections, and a round-trip write
must not clobber prose it doesn't understand.

## Machine state block

A single HTML-comment block anywhere in the file carries the authoritative
state as YAML:

```md
<!-- EVIDENCE-LOOP:STATE
version: 1
experiment: E001
state: RESULT_REVIEWED
-->
```

This is the only block `evidence-loop` parses for control state. Everything
else in the file is prose that `evidence-loop` preserves byte-for-byte
across writes, except for the specific sections it owns (below).

`version` is the board schema version. `evidence-loop` refuses to operate on
a board whose version it doesn't recognize rather than guessing.

## Per-experiment record

Each experiment also gets a YAML sidecar the CLI reads and writes directly:

```text
.evidence-loop/experiments/E001.yml
```

```yaml
id: E001
hypothesis: "H1 — decomposition improves retrieval"
state: RESULT_REVIEWED
created_at: 2026-08-31T00:00:00Z
raw:
  path: evidence/E001/raw/result.json
  baseline_commit: 9d7c1c7
  hash: sha256:...
  components: 1
  rows: 286
  terminus: verified
  verified_at: 2026-08-31T00:05:00Z
result:
  classification: DEGENERATE
  mechanism: UNEXERCISED
  hypothesis_status: UNTESTED
  recorded_at: 2026-08-31T00:10:00Z
  reviewed_at: 2026-08-31T00:12:00Z
  reviewer: reviewer@example.com
  review_kind: INDEPENDENT
  review_notes: "Spot-checked the observation against the raw artifact; no drift."
artifact:
  committed_at: 2026-08-31T00:15:00Z
  commit_sha: abc1234   # absent/null if committed with --local
closure:
  status: INCONCLUSIVE
  recorded_at: 2026-08-31T00:20:00Z
  reviewed_at: 2026-08-31T00:22:00Z
  reviewer: reviewer@example.com
  review_kind: INDEPENDENT
  review_notes: null
```

This sidecar is the source of truth for `evidence-loop`. `BOARD.md`'s
prose sections (`## Evidence`, `## Result`, `## Closure`, `## History`) are
regenerated from it on every write, so the sidecar and the board can never
silently diverge. Prose sections outside the ones `evidence-loop` owns
(`## Hypothesis`, `## Next Action`'s free text, any section a human adds)
pass through unchanged.

## Vocabularies

```text
classification:      CONCLUSIVE | INCONCLUSIVE | DEGENERATE
mechanism:            EXERCISED | UNEXERCISED
hypothesis_status:    SUPPORTED | REFUTED | UNTESTED
closure.status:       CONFIRMED | REFUTED | INCONCLUSIVE
review_kind:          INDEPENDENT | SELF
```

## Validation rules

- `mechanism: UNEXERCISED` + `hypothesis_status: REFUTED` is rejected. A
  mechanism that never ran cannot refute anything.
- `classification: DEGENERATE` + `hypothesis_status` other than `UNTESTED`
  is rejected unless `result --override-flag --override-reason "..."` is
  passed explicitly at record time. The override is always a deliberate,
  visible CLI argument — never something the record ends up with by editing
  the sidecar directly, and never a default.
- Once `artifact` is set (whether `commit_sha` is present or the artifact
  was pinned with `--local`), a `verify` run that computes a different
  `raw.hash` at `raw.path` fails validation instead of updating the record.
  The artifact is now historical; a changed file is new evidence and
  belongs to a new experiment.

## Evidence bundles

`raw.path` may name a directory instead of a single file, for evidence that
naturally spans several related raw files. `verify` hashes every file under
it (sorted by relative path, each file's own hash combined with its path)
into one `raw.hash`, so the combined hash changes if any file's content,
name, or set membership changes -- the same immutability guarantee a single
file gets. `raw.components` becomes the file count and `raw.terminus`
becomes `bundle`; `raw.rows` is unused (`0`) since a bundle has no row
concept. `commit-artifact` stages and commits the whole directory in one
commit. Use a single file when the evidence naturally is one file --
bundling only when the raw evidence itself is multiple genuinely related
files (e.g. an installer's output plus a separate verification pass'
output), not as a place to stash unrelated files together.

## Local-only artifacts

`commit-artifact --local` satisfies the `ARTIFACT_COMMITTED` transition by
pinning the artifact's hash for integrity without writing it into the
subject repository's git history -- `artifact.commit_sha` is left absent
instead of holding a SHA. This exists because the project being
experimented on and the place experiment evidence should live are not
always the same thing: raw command output, environment details, or other
experiment-specific material may not belong in a repository other people
track for unrelated reasons. `--local` is an explicit, visible choice (the
board renders it distinctly, never as if an independent git commit
happened) -- it does not change the default, which is still to commit,
consistent with the historical-evidence invariant for any experiment where
the subject repository is genuinely where the evidence belongs.

## Result → closure status

`hypothesis_status` (set at `result`) and `closure.status` (set at `close`)
are two separate fields with no automatic mapping enforced by the tool — the
person or agent running `close` states the closure status directly, and it
should follow from the result the same way `hypothesis_status` does:

```text
hypothesis_status: SUPPORTED               -> closure.status: CONFIRMED
hypothesis_status: REFUTED                 -> closure.status: REFUTED
hypothesis_status: UNTESTED (or otherwise
  inconclusive/degenerate evidence)        -> closure.status: INCONCLUSIVE
```

`evidence-loop` does not currently reject a `close` call that contradicts
this table — it is a convention to follow, not a validated invariant, since
enforcing it mechanically would require the tool to interpret whether a
`close` call's free-text `established`/`not_established` reasoning actually
justifies the chosen status.

## Review independence

`review_kind` on a reviewed `result` or `closure` distinguishes an
independent review (`INDEPENDENT`, the default) from a self-review
(`SELF`, set only when `review-result`/`review-closure` is run with
`--self`). Self-review does not block the transition — the loop keeps
moving — but the distinction must stay visible: `BOARD.md` renders a
`### Reviewed By` line under `## Result`/`## Closure` stating which kind of
review happened, and `evidence-loop gate`'s text and `--json` output both
surface it. A self-reviewed experiment can still reach `HYPOTHESIS_GATE`;
it just can't look like an independently-reviewed one did.

`--notes` on either review command is stored as its own `review_notes`
field and rendered as its own `### Review Notes` section — it is never
appended into `interpretation` or `remaining_questions`, which stay exactly
what was recorded at `result`/`close` time.

## Example rendered `BOARD.md`

```md
# Evidence Board

<!-- EVIDENCE-LOOP:STATE
version: 1
experiment: E001
state: RESULT_REVIEWED
-->

## Current Experiment

E001

## Hypothesis

H1 — decomposition improves retrieval...

## Next Action

Commit raw artifact.

## Evidence

- Artifact: `evidence/E001/raw/result.json`
- Baseline: `9d7c1c7`
- Components: 1
- Rows: 286
- Terminus: verified

## Result

### Classification

DEGENERATE

### Mechanism Exercised

NO

### Hypothesis Status

UNTESTED

### Reviewed By

INDEPENDENT -- reviewer@example.com

## Closure

Pending

## History

- RAW_PENDING -> RAW_VERIFIED
- RAW_VERIFIED -> RESULT_RECORDED
- RESULT_RECORDED -> RESULT_REVIEWED
```
