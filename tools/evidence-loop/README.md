# evidence-loop

A deterministic Rust CLI that enforces an evidence bookkeeping and closure
protocol for repeated, hypothesis-driven experiments run by AI agents (or
humans). It exists so that a mechanism that never ran can't quietly become
"the hypothesis was refuted," and so that a result can't be interpreted
before it's actually been verified and reviewed.

```
AI Agent  -->  SKILL.md  -->  evidence-loop CLI  -->  BOARD.md + Git
```

The agent does the reasoning. `evidence-loop` enforces the protocol:
one state transition per invocation, no skipping states, no rewriting
committed evidence, and no silently treating an unexercised mechanism as a
refutation.

## Install

```bash
cargo install evidence-loop
```

## Use

```bash
evidence-loop init
evidence-loop new "H1 -- some falsifiable claim"
evidence-loop next --json   # the single next legal action, and nothing else
```

Every experiment moves through one state at a time:

```
RAW_PENDING -> RAW_VERIFIED -> RESULT_RECORDED -> RESULT_REVIEWED ->
ARTIFACT_COMMITTED -> CLOSURE_RECORDED -> CLOSURE_REVIEWED -> HYPOTHESIS_GATE
```

`evidence-loop` is the sole source of truth for what's legal next; `next --json`
is meant to be the primary interface for an agent driving this loop.

## Full documentation

This crate is one part of [Magithar/skills](https://github.com/Magithar/skills):

- [`docs/evidence-loop/protocol.md`](https://github.com/Magithar/skills/blob/main/docs/evidence-loop/protocol.md) -- the protocol and its invariants
- [`docs/evidence-loop/state-machine.md`](https://github.com/Magithar/skills/blob/main/docs/evidence-loop/state-machine.md) -- the full state/transition table
- [`docs/evidence-loop/board-format.md`](https://github.com/Magithar/skills/blob/main/docs/evidence-loop/board-format.md) -- the `BOARD.md`/sidecar schema
- [`docs/evidence-loop/soak-notes.md`](https://github.com/Magithar/skills/blob/main/docs/evidence-loop/soak-notes.md) -- findings from real usage, and why the feature set stops where it does
- [`skills/engineering/evidence-loop/SKILL.md`](https://github.com/Magithar/skills/blob/main/skills/engineering/evidence-loop/SKILL.md) -- the agent-facing instructions for driving this CLI

## License

MIT
