# State Machine

`evidence-loop` tracks one experiment's evidence lifecycle as a linear sequence
of states, plus a two-way branch at the end. States never skip; every
transition is a single named command.

## States

| State | Meaning |
|---|---|
| `NEW` | Experiment created, no evidence work started. |
| `RAW_PENDING` | Raw evidence must be produced or located. |
| `RAW_VERIFIED` | Raw artifact has been mechanically checked. |
| `RESULT_RECORDED` | A result has been written from the verified evidence. |
| `RESULT_REVIEWED` | The result has been independently reviewed. |
| `ARTIFACT_COMMITTED` | The raw artifact has been committed to Git, separately from interpretation. |
| `CLOSURE_RECORDED` | Experiment closure has been written. |
| `CLOSURE_REVIEWED` | Closure has been reviewed. |
| `HYPOTHESIS_GATE` | Evidence is now allowed to influence the next hypothesis. |
| `TERMINATED` | The evidence loop for this experiment is complete. |

`HYPOTHESIS_GATE` is a terminal state for the experiment itself. A new
experiment (`evidence-loop new`) is a separate object on the board, not a
state this experiment transitions into.

## Transitions

```text
NEW               --start-->            RAW_PENDING
RAW_PENDING       --verify-->            RAW_VERIFIED
RAW_VERIFIED      --result-->            RESULT_RECORDED
RESULT_RECORDED   --review-result-->     RESULT_REVIEWED
RESULT_REVIEWED   --commit-artifact-->   ARTIFACT_COMMITTED
ARTIFACT_COMMITTED --close-->            CLOSURE_RECORDED
CLOSURE_RECORDED  --review-closure-->    CLOSURE_REVIEWED
CLOSURE_REVIEWED  --gate-->              HYPOTHESIS_GATE
HYPOTHESIS_GATE   --terminate-->         TERMINATED
```

Every other `(state, command)` pair is illegal and the CLI rejects it with a
non-zero exit and an explanation of what state the experiment is actually in.

One command call performs at most one transition. A command never chains
transitions on behalf of the caller, even when every precondition for the
next state already holds — the caller (human or agent) must invoke the next
command explicitly.

## Invariants enforced by every transition

1. **Evidence invariant** — a result cannot be recorded before its raw
   artifact is verified; a closure cannot be recorded before its result is
   reviewed. Interpretation cannot outrun verification.
2. **Mechanism invariant** — `mechanism: UNEXERCISED` never implies
   `hypothesis_status: REFUTED`. The validator rejects that combination
   outright (see `board-format.md`).
3. **Transition invariant** — states cannot be skipped. `RESULT_RECORDED ->
   CLOSURE_RECORDED` is illegal even if the caller believes the intermediate
   steps are unnecessary.
4. **Historical evidence invariant** — once `commit-artifact` has run, the
   committed raw artifact's hash is fixed on the board. A later `verify` run
   that finds a different hash at the recorded path is a validation failure,
   not a silent update.
5. **Hypothesis gate invariant** — `gate` only succeeds when raw is verified,
   the result is reviewed, the artifact is committed, and the closure is
   reviewed. There is no path to `HYPOTHESIS_GATE` that skips review.

## Rejected transition example

```text
$ evidence-loop close E001
error: illegal transition

  experiment E001 is in state RESULT_RECORDED
  `close` requires ARTIFACT_COMMITTED

  next legal command: review-result E001
```
