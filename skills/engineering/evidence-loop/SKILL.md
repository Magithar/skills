---
name: evidence-loop
description: Enforces a deterministic evidence lifecycle for repeated experiments — raw artifact verified, result recorded, result reviewed, artifact committed to git, closure recorded, closure reviewed — before any hypothesis conclusion is allowed. Use when running an experiment loop where a hypothesis is being tested against evidence and premature or overreaching conclusions are a real risk.
---

# evidence-loop — Evidence Bookkeeping and Closure Enforcement

**Trigger:** Use this skill when you are running (or resuming) an experiment loop that produces raw
evidence, and a hypothesis conclusion should not get ahead of that evidence — e.g. "run the next
experiment", "record this result", "is H1 supported yet", or when a project has a `BOARD.md` created by
`evidence-loop init`.

**Do NOT activate for:** one-off analysis with no hypothesis to protect, or a project that isn't using
`evidence-loop` (no `BOARD.md` in the project root or an ancestor directory).

## The rule that governs everything below

The `evidence-loop` CLI (in `tools/evidence-loop/` of this repo, built as the `evidence-loop` binary) is
the only source of truth for what state an experiment is in and what you may do next. You do not infer
the workflow — you ask the tool.

**One invocation of this skill performs at most one state transition.** Run the one command `next`
tells you is legal, then stop and report what happened. Do not chain multiple `evidence-loop` commands
in a row to walk further down the lifecycle in a single turn, even if you believe every later
precondition already holds.

## Procedure

1. Run `evidence-loop next --json` (or `evidence-loop status --json` if you don't know the current
   experiment). This returns the current experiment, its state, and the one command that is legal next,
   along with `required_facts` (what you must establish before running it) and `forbidden` (what not to
   do while running it).
2. If `next_command` is `null`, the experiment is at `HYPOTHESIS_GATE` — its evidence is fully closed out
   and reviewed. A new hypothesis is a new experiment (`evidence-loop new "<hypothesis>"`), not a
   transition of this one. Stop and report the gate's decision; do not silently start a new experiment
   unless asked to.
3. Otherwise, perform exactly the action `next_command` names, satisfying every item in
   `required_facts`, and run the corresponding `evidence-loop` command yourself:

   | Command | What it does |
   |---|---|
   | `verify <id> <path>` | Mechanically checks a raw artifact (hash, structure, git baseline). Never call this before the artifact actually exists. |
   | `result <id> --classification --mechanism --hypothesis-status --observation --interpretation` | Records a result from verified evidence. `classification` is `CONCLUSIVE`/`INCONCLUSIVE`/`DEGENERATE`; `mechanism` is `EXERCISED`/`UNEXERCISED`; `hypothesis-status` is `SUPPORTED`/`REFUTED`/`UNTESTED`. |
   | `review-result <id> --reviewer <name>` | Records that the result was independently reviewed. |
   | `commit-artifact <id>` | Commits the raw artifact to git, separately from interpretation. Fails if the artifact changed since verification — that means it's new evidence, not this one. |
   | `close <id> --status --established --not-established --remaining-questions` | Records closure. `status` is `CONFIRMED`/`REFUTED`/`INCONCLUSIVE`. |
   | `review-closure <id> --reviewer <name>` | Records that closure was independently reviewed. |
   | `gate <id>` | Confirms every prior gate condition and opens the hypothesis gate. Only this command may end an experiment's evidence loop. |

4. If the CLI rejects the command (illegal transition, or a semantic validation error), that rejection is
   the correct behavior — do not retry with different flags to force it through, and do not hand-edit
   `BOARD.md` or the `.evidence-loop/experiments/<id>.yml` sidecar to route around it. Read the error,
   fix the actual problem (wrong state, missing artifact, an inconsistent classification), and re-run.
5. Report the transition and stop. On the next invocation, start again from step 1 — do not rely on
   memory of where the experiment was.

## Rules you must never route around

- **Never treat an `UNEXERCISED` mechanism as a refutation.** The CLI rejects
  `--mechanism UNEXERCISED --hypothesis-status REFUTED` outright; that rejection is protecting a real
  invariant, not a bug to work around.
- **Never claim a `DEGENERATE` result supports or refutes the hypothesis** without an explicit,
  human-reviewable `--override-flag --override-reason`. If you don't have a genuine reason a degenerate
  run still bears on the hypothesis, the honest result is `--hypothesis-status UNTESTED`.
- **Never rewrite historical raw evidence.** If a committed artifact needs to change, that's a new
  experiment (`evidence-loop new`), not an edit to the old one.
- **Never skip states**, even when you're confident the intermediate step is a formality. `next --json`
  is the only thing that tells you what's legal.

See `docs/evidence-loop/protocol.md`, `docs/evidence-loop/state-machine.md`, and
`docs/evidence-loop/board-format.md` in this repo for the full specification.
