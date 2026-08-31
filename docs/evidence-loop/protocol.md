# Evidence Loop Protocol

`evidence-loop` is a Git-native evidence bookkeeping and closure system for
AI agents running repeated experiments. It exists so that an agent's
*interpretation* of evidence can never get ahead of the evidence itself.

```text
AI AGENT  --SKILL.md-->  evidence-loop CLI (Rust)  --owns-->  BOARD.md + evidence/ + Git
```

The Rust CLI is not an agent. It is a deterministic referee: it knows the
legal states, the legal transitions between them, and the semantic rules
that make a recorded result or closure valid. It performs no reasoning about
what evidence *means* — that stays with the agent, and only within the
window the protocol allows it.

## Core principle

> AI performs reasoning. Rust enforces the protocol. `BOARD.md` stores
> current state. Git stores historical truth.

## What the protocol guarantees

- **Evidence invariant** — verification precedes recording, review precedes
  closure. See `state-machine.md`.
- **Mechanism invariant** — an experiment whose mechanism never ran
  (`UNEXERCISED`) can never be recorded as having refuted its hypothesis.
  A degenerate experiment is not evidence against anything.
- **Transition invariant** — states cannot be skipped; illegal transitions
  are rejected with the actual next legal command.
- **Historical evidence invariant** — a committed raw artifact is fixed. A
  changed artifact is new evidence, not a silent edit to old evidence.
- **Hypothesis gate invariant** — no experiment can justify a new hypothesis
  until its raw evidence is verified, its result reviewed, its artifact
  committed, and its closure reviewed.
- **Agent transition invariant** — one agent invocation performs at most one
  state transition. An agent cannot autonomously walk the entire lifecycle
  in a single turn.

See `state-machine.md` for the full state list and transition table, and
`board-format.md` for how state is persisted in `BOARD.md` and the
per-experiment sidecar.

## What the protocol deliberately does not do

- It does not decide whether a hypothesis is true. That is the agent's job,
  constrained to run only after the gate conditions above are met.
- It does not know what a "good" experiment looks like for any particular
  domain. Domain-specific verification (JSON shape, CSV row counts,
  benchmark thresholds) is out of scope for v1 and left to later domain
  adapters — the state machine itself is domain-independent.
- It does not orchestrate agents. `evidence-loop` is a CLI a human or an
  agent invokes one command at a time; there is no built-in agent loop in
  this version.

## Repository layout

This repository (`Magithar/skills`) hosts the reusable protocol and
tooling:

```text
skills/engineering/evidence-loop/SKILL.md   — thin agent-facing instructions
tools/evidence-loop/                         — the Rust CLI
docs/evidence-loop/                          — this protocol
```

A project actually running experiments carries its own state, separate from
this repo:

```text
project/
├── BOARD.md
├── evidence/
│   └── E001/
│       └── raw/
└── .evidence-loop/
    └── experiments/
        └── E001.yml
```

## Versioning

The board schema carries a `version` field (see `board-format.md`).
Breaking changes to the board format bump it; `evidence-loop` refuses to
operate on a board with an unrecognized version rather than guessing at its
shape.
