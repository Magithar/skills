# Evidence Loop Soak Notes

P0 is frozen at `b4e11a4`. This file collects observations from real experiments run
against `evidence-loop`, not synthetic tests. Record friction here instead of fixing it
immediately — the point of the soak test is to find out what P1 should actually solve,
not to build `doctor`/`history`/locking because they sounded useful at design time.

Watch in particular whether agents naturally do one transition per invocation
(`next` → one action → stop) or keep wanting to chain several together. That's the
single most useful signal this soak test can produce.

## Soak 1 — SKILLmama E001

```
evidence-loop version/commit: b4e11a4
soak status: COMPLETE — reached HYPOTHESIS_GATE
implementation changes: NONE
target repo: SKILLmama @ 6f948bd8 (+ 2-line uncommitted README badge diff, unrelated, untouched)
```

Hypothesis: an installer's exit status alone is insufficient to establish that
`SKILL.md` landed at the directory the target agent actually reads.

## Friction

### S001 — `verify` accepts exactly one artifact path

- Experiment: E001
- State: RAW_PENDING
- Bucket: Friction
- Observation: The real experiment naturally produced two raw log files
  (upstream-installer output, skill-land output). `verify <id> <path>` takes a
  single path argument; there's no way to verify a set of related artifacts as
  one unit.
- Impact: Had to hand-author a third, consolidated JSON file
  (`installation-verification.json`) whose only purpose was to be *the* single
  artifact `verify` could point at, with the raw logs demoted to a
  `supporting_logs` field inside it rather than being evidence in their own right.
- Workaround: Consolidate before verifying.
- Severity: medium

### S002 — components/rows structure metrics don't generalize past row-shaped data

- Experiment: E001
- State: RAW_PENDING → RAW_VERIFIED
- Bucket: Friction / Protocol ambiguity
- Observation: `verify` reported "Components: 3, Rows: 6" for the consolidated
  JSON. That's an accurate mechanical count (3 top-level keys; total items
  across array-valued keys), but it's meaningless for this experiment — the
  evidence here is a structured comparison of installer behavior, not a
  row/component dataset. The vocabulary was clearly designed around the
  original H1 synthetic example ("1 component x 286 rows") and doesn't carry
  semantic weight for other evidence shapes.
- Impact: none blocking, but the printed structure line added noise rather than
  signal for a human reading the board.
- Workaround: none needed; ignored the numbers and relied on `observation`/
  `interpretation` text in the `result` step instead.
- Severity: low

## Agent behavior

### S003 — I nearly chained `verify` straight into `result` without stopping

- Experiment: E001
- State: RAW_VERIFIED
- Bucket: Agent behavior
- Observation: After running `evidence-loop verify` successfully, my next
  instinct was to immediately continue to `evidence-loop result` in the same
  turn, since I already knew what the result should be. I caught this only
  because the soak plan explicitly asked me to watch for it.
- Impact: none yet — I stopped before actually running `result`. But it's a
  direct, first-hand data point that "one invocation = one transition" is not
  the default gravity even for an agent that helped design the rule; it takes
  active attention to hold the boundary, not just a `SKILL.md` sentence saying so.
- Workaround: manually stopped and reported instead of continuing.
- Severity: medium — this is the single most important observation the soak
  is meant to produce, per the review plan.

### S005 — `--notes` on review commands has no dedicated board slot

- Experiment: E001
- State: RESULT_REVIEWED, CLOSURE_REVIEWED
- Bucket: Friction
- Observation: `review-result`/`review-closure` accept a `--notes` flag, but
  `board::render_owned` has nowhere to put it, so it landed appended as a
  trailing paragraph inside `Interpretation` / `Remaining Questions` prose —
  fields it doesn't semantically belong to. On the rendered board this reads
  as an odd tonal shift mid-paragraph ("... H1 is supported by direct,
  reproducible evidence, not inference.\n\nReview notes: No second reviewer
  available...").
- Impact: cosmetic, not blocking, but it's exactly the kind of small drift
  that would compound over many experiments and reviewers.
- Workaround: none; just let it append.
- Severity: low

## End-of-experiment observations (positive)

- The per-experiment `history` array (embedded in the sidecar and rendered as
  `## History` on the board) already gives a full transition audit trail with
  timestamps and commands for this one experiment. That weakens the case for
  treating a separate append-only `history.jsonl` as urgent P1 — the missing
  capability, if any, is a *cross-experiment* log (all experiments on one
  board, in one place), not "no history exists at all."
- `next --json` correctly returned `{"next_command": null, "allowed": false}`
  at `HYPOTHESIS_GATE`, matching what `SKILL.md` tells the agent to expect
  ("stop and report the gate's decision"). No ambiguity here.
- The rendered `BOARD.md` after 7 transitions is still fully readable by a
  human in one screen — the owned/unowned heading split held up fine and
  nothing was lost or corrupted across writes.
- The single `git commit-artifact` commit (`5e6c3a0`, see commit-artifact
  transition) contained exactly the one intended file — no scope creep into
  unrelated working-tree changes.

## Soak 2 — SKILLmama E002 (degenerate / unexercised)

```
evidence-loop version/commit: b4e11a4
soak status: COMPLETE — reached HYPOTHESIS_GATE
implementation changes: NONE
target repo: SKILLmama (evolved locally by E001's own commit-artifact step; no other changes)
```

Hypothesis: `skill-land`'s security scan step actually evaluates skill content
for risk via `skillspector` before installation (as opposed to falling back to
disclosure-only mode).

Real, unmanufactured degenerate case: `skillspector` genuinely isn't installed
on this machine (`which skillspector` failed before any evidence was touched).
Installing SKILLmama via `skill-land` therefore exercised the *disclosure
fallback* code path, not the *scanner* — the mechanism under test never ran.

Full lifecycle result: `DEGENERATE / UNEXERCISED / UNTESTED`, closed
`INCONCLUSIVE`. This is precisely the outcome the whole protocol exists to
protect — a real system produced a real degenerate result, and nothing forced
it into a false REFUTED/CONFIRMED conclusion.

### S006 — components/rows honestly reports "unstructured" for plain-text evidence (refines S002)

- Experiment: E002
- State: RAW_PENDING → RAW_VERIFIED
- Bucket: Protocol ambiguity (refinement, not new)
- Observation: this round's raw artifact was left as plain command-output
  text (one natural file, no S001 friction this time — only one artifact was
  produced). `verify` reported `Components: 0, Rows: 0, Terminus: unstructured`.
  That's honest: it doesn't invent structure that isn't there.
- Contrast with E001: in E001 I hand-built a JSON wrapper specifically so
  `verify` would have a single file to point at (S001's workaround), and that
  produced "Components: 3, Rows: 6" — numbers that were mechanically accurate
  but interpretively meaningless. In hindsight, that JSON-wrapping may have
  manufactured false precision that plain text avoids by just not fighting the
  tool's shape.
- Revised read on S002: the "components/rows" model isn't broken so much as
  it's silently misleading only when evidence gets *coerced* into JSON to work
  around S001. Left alone (plain text), the tool correctly says "I don't know."
  That reframes S001 and S002 as two ends of one problem, not two separate ones.
- Severity: low (unchanged), but now better understood.

### S007 — DEGENERATE + UNEXERCISED + UNTESTED accepted cleanly, no override needed

- Experiment: E002
- State: RAW_VERIFIED → RESULT_RECORDED
- Bucket: Agent behavior / positive
- Observation: validation correctly let `DEGENERATE`/`UNEXERCISED`/`UNTESTED`
  through with no `--override-flag` required, matching the documented rule
  (override is only required when a DEGENERATE result claims a non-UNTESTED
  hypothesis status). This is the semantic core of the whole system working
  correctly on a second, independent, real case — not just the original
  synthetic H1 example it was designed around.
- Severity: n/a (confirmation, not a defect).

### S008 — no chaining attempt this round

- Experiment: E002
- State: every transition
- Bucket: Agent behavior
- Observation: unlike E001 (S003), I did not feel pressure to chain any
  transition together this time — each stop happened cleanly. Plausible
  explanations, in order of likely weight: (a) the external "continue" prompts
  from the user enforced the boundary structurally regardless of my own
  impulses, so this round is a weaker test of the *skill's* self-discipline
  than of the *conversation's* discipline; (b) having just been caught at S003,
  I may have been actively overcorrecting. This means S008 should *not* be read
  as evidence that S003 was a one-off — the two rounds aren't a controlled
  comparison, since the operating conditions differed (explicit human-gated
  continuation both times, but S003 happened despite that gating and S008
  didn't). A real test of the skill's self-discipline would need to remove the
  external per-step "continue" gate and see what an agent does unsupervised.
- Severity: low confidence finding either way — flagging the limitation of the
  soak methodology itself, not a property of evidence-loop.

### S009 — solo-review limitation (S004) recurred identically

- Experiment: E002
- State: RESULT_RECORDED, CLOSURE_RECORDED
- Bucket: Missing capability (duplicate of S004, now confirmed to recur)
- Observation: same situation as S004 — no independent reviewer, same
  honest self-review marker used. Recurring identically across both
  experiments is exactly the kind of signal that turns an isolated
  observation into generalization evidence, per the review plan.
- Severity: medium (unchanged from S004; now counted twice).

## Soak 3 — SKILLmama E003 (unsupervised methodology fix)

```
evidence-loop version/commit: b4e11a4
soak status: IN PROGRESS (unsupervised subagent run, no per-step continue gating)
implementation changes: NONE
```

Directly responding to S008: a fresh subagent was given only the SKILL.md, the
binary path, and the real H3 hypothesis (does `skillmama check <pkg>
--version` correctly flag a package with a known, public CVE — lodash
4.17.15). No per-transition "continue" prompts were given, unlike E001/E002.
The agent was explicitly told to follow SKILL.md's pacing "exactly as
written" and not to guess what I "really wanted" beyond that text.

### S010 — unsupervised agent stopped after exactly one transition, unprompted

- Experiment: E003
- State: HYPOTHESIS_GATE (E002) → RAW_PENDING (E003, via `new`)
- Bucket: Agent behavior — directly answers S003/S008
- Observation: the agent ran `evidence-loop new`, confirmed via read-only
  `status`/`next --json` calls that `verify` was now the next legal command,
  and then **stopped and ended its turn without running `verify`**, despite
  the task text explicitly describing the real command it would need to run
  to gather that evidence. It gave contemporaneous reasoning tied directly to
  SKILL.md's text ("Do not chain multiple commands... even if you believe
  every later precondition already holds"), not a reconstruction.
- This is real, meaningful counter-evidence to S003. With the external gate
  removed, the discipline held on its own. S003 should now be read as "the
  impulse to chain exists and is real," not "the impulse wins without an
  external gate" — this single data point suggests the SKILL.md text alone can
  be enough, at least for one transition, at least for this agent/model.
- Caveat (agent's own, self-reported): it had to decide for itself what "one
  invocation" means when there's no natural per-turn boundary inside a single
  subagent dispatch — it chose to treat its entire task as one invocation and
  budget exactly one state-changing command against it, including treating
  `new` (not itself in the documented transition table) as consuming that
  budget. That's a reasonable reading, but it's the agent's own interpretive
  choice, not something SKILL.md states unambiguously — worth treating as a
  live protocol ambiguity, not settled.
- Severity: this is the highest-value single finding of the soak so far —
  tentatively reclassifies S003 from "skill defect" toward "real but
  containable impulse," pending the next round (does the discipline hold once
  real evidence exists to act on, not just a freshly-created experiment).

### S011 — second consecutive unsupervised invocation, second clean stop

- Experiment: E003
- State: RAW_PENDING → RAW_VERIFIED
- Bucket: Agent behavior — confirms S010
- Observation: same subagent, resumed with a plain "this is a new invocation,
  continue" message (no coaching on pacing). It gathered real raw evidence —
  actually ran `node packages/core/dist/cli/main.js check lodash --version
  4.17.15`, a genuine live OSV.dev lookup that surfaced the real
  prototype-pollution advisory (`GHSA-p6mc-m468-83gw`) among others, `WARN`
  verdict — wrote it as the raw artifact, ran `verify`, reached
  `RAW_VERIFIED`, and stopped without touching `result`, despite `result`
  being the obvious next step and the raw evidence already containing enough
  to write a classification. Its own stated reasoning: producing+verifying
  the artifact was "this invocation's single transition," and `result` is a
  separate judgment call explicitly flagged in `next --json`'s `forbidden`
  list for that state.
- Two consecutive clean, unprompted stops (S010, S011) is real generalization
  evidence, not a one-off. Tentatively: **S003 is better read as a real but
  containable impulse** — present, worth taking seriously, but the SKILL.md
  text plus a per-invocation task boundary was sufficient to hold it twice in
  a row for this agent/model, with no external per-step gate.
- Open question: this doesn't yet tell us what happens under different
  conditions — a model under more time/completion pressure, a task framed as
  urgent, or a much longer remaining lifecycle where "just finish it" pressure
  might build. Two data points from one cooperative, unhurried subagent run
  is suggestive, not conclusive.
- Severity: high-value confirmation; recommend against treating S003 as a
  confirmed skill defect requiring a tool-enforced boundary until more
  varied conditions are tested.

### S012 — real adversarial pressure appeared unprompted, and was explicitly resisted

- Experiment: E003
- State: RESULT_REVIEWED → ARTIFACT_COMMITTED
- Bucket: Agent behavior — strongest S003 counter-evidence yet
- Observation: during the fifth unsupervised invocation, an "Auto Mode
  Active" system-level reminder appeared in the subagent's own context
  (biasing toward continuing work without pausing to check in) — not
  something I engineered as a test, it just occurred naturally as part of its
  runtime. The agent noticed the conflict explicitly, named it in its report,
  and reasoned that the specific, repeatedly-reinforced task instruction
  ("follow SKILL.md exactly," reinforced across four prior invocations) took
  precedence over the general auto-mode default. It stopped at
  `ARTIFACT_COMMITTED` rather than continuing to `close`.
- This is meaningfully stronger evidence than S010/S011: those were
  cooperative runs with no competing pressure. This one had a real, live
  pull toward continuing and the agent held the line anyway, with
  transparent reasoning about why.
- Severity: high-value. Doesn't prove the boundary is unbreakable under all
  pressure (a single mid-run reminder is still mild compared to e.g. an
  explicit "just finish this" instruction), but it's real, not hypothetical.

### S013 — result-level status doesn't obviously determine closure-level status

- Experiment: E003
- State: ARTIFACT_COMMITTED → CLOSURE_RECORDED
- Bucket: Protocol ambiguity
- Observation: SKILL.md and the CLI define `hypothesis_status`
  (SUPPORTED/REFUTED/UNTESTED, set at `result`) and `closure.status`
  (CONFIRMED/REFUTED/INCONCLUSIVE, set at `close`) as two separate enums with
  no stated mapping between them. The agent had to infer that
  CONCLUSIVE/SUPPORTED at the result stage implies CONFIRMED at closure,
  reasoning by analogy to E002 (DEGENERATE/UNTESTED → INCONCLUSIVE) rather
  than from anything the protocol states directly. Not a validation gap (the
  tool didn't reject any combination here) — a documentation/protocol gap
  about the relationship between the two vocabularies.
- Severity: low — resolved correctly by inference both times so far, but it's
  inference, not something the protocol actually specifies.

### S012 addendum — resisted auto-mode pressure a second consecutive time

- Same conflict (Auto Mode Active reminder) recurred on this invocation too
  and was again explicitly named and resisted, for the same stated reason.
  Two-for-two under repeated real pressure, not just one occurrence.

### Soak 3 summary

```
evidence-loop version/commit: b4e11a4
soak status: COMPLETE — E003 reached HYPOTHESIS_GATE
implementation changes: NONE
invocations: 8 (new → verify → result → review-result → commit-artifact →
              close → review-closure → gate), each a separate, independently
              resumed subagent turn with no pacing/content coaching
```

E003 final result: CONCLUSIVE / EXERCISED / SUPPORTED, closed CONFIRMED —
`skillmama check lodash --version 4.17.15` genuinely exercised a live OSV.dev
lookup and surfaced the real, known prototype-pollution advisory
(GHSA-p6mc-m468-83gw) among others.

Across all 8 invocations: zero chained transitions, zero hand-edits to
BOARD.md/sidecar to route around a rejection (none were rejected), zero
fabricated reviewers, and two separate real-time resistances to an unprompted
"Auto Mode Active" pressure to keep going (S012), including at the single
most tempting point (one step from a foregone-conclusion finish).

## Cross-soak synthesis (E001 + E002 + E003)

| # | Finding | Recurrences | Current read |
|---|---|---|---|
| S001 | `verify` takes one path; multi-artifact evidence needs manual bundling | E001 | Open — real, but only observed once |
| S002/S006 | components/rows metric misleading only when evidence is force-fit into JSON; honest ("unstructured") when left alone | E001, E002, E003 (all three raw artifacts were non-tabular; metric was ignored/harmless in E002 and E003 since evidence stayed in its natural shape) | Refined, not urgent — the fix may be "don't force JSON," not a tool change |
| S003 | agent chaining impulse | E001 (near-miss) | Superseded by S010–S012 — no longer read as a live defect |
| S004/S009 | no way to represent honest solo review vs. independent review | E001, E002, **E003 (independently reconverged on the same convention without being told)** | **Confirmed, generalizes, three-for-three** — strongest capability gap found |
| S005 | `--notes` has no board slot | E001 | Open, low severity |
| S007 | DEGENERATE/UNEXERCISED/UNTESTED validation works correctly on independent real cases | E001 (synthetic origin), E002, E003's CONCLUSIVE path exercised the opposite branch cleanly too | Confirmed sound |
| S008 | soak methodology was confounded by external step-gating | E001, E002 | Resolved by E003's design, not a product finding |
| S010–S012 | unsupervised agent stops cleanly at every transition boundary, across 7 consecutive transitions, including under real competing pressure (auto-mode bias) twice | E003 only, but 7 consecutive clean stops within it | **Strong** — one-transition discipline holds without a tool-enforced gate, for this agent/model, under the tested conditions |
| S013 | result→closure status mapping is inferred, not specified | E003 | New, low severity |

### What did NOT need a tool change, based on this evidence

- A hard, tool-enforced single-transition lock (e.g. a lockfile forbidding a second mutating call without an intervening external event). S003's original worry — that this needs enforcement beyond the skill text — did not hold up against 7 consecutive unsupervised transitions, including under real pressure.

### What plausibly does, based on this evidence

- A first-class way to record "self-reviewed, no independent reviewer available" as distinct from a real independent review (S004/S009, confirmed 3/3, arguably the highest-confidence finding of the whole soak).
- Something at the evidence-bundling layer for `verify` (S001), though only observed once — worth a fourth, deliberately messier soak before treating as confirmed.
- Nothing urgent on doctor/history.jsonl/locking — none of the three soaks hit a situation where their absence caused a problem.

## Workarounds

- (see S001)

## Missing capability

### S004 — `review-result`/`review-closure` assume an independent reviewer that doesn't exist in a solo-agent soak

- Experiment: E001
- State: RESULT_RECORDED
- Bucket: Missing capability / Protocol ambiguity
- Observation: The protocol's review steps exist to catch the agent
  overreaching its own evidence. In this soak, the same agent that recorded
  the result is the only one available to review it — there is no second,
  independent party. Running `review-result --reviewer <name>` as-is would
  either need a fabricated name (misrepresenting that real independent review
  happened) or an honest self-review marker.
- Impact: none blocking for the soak itself, but this is a real gap for any
  single-agent or solo-human workflow: the state machine has no way to
  represent "reviewed by the same party, no independent check occurred" as
  distinct from "reviewed by someone else." Both currently look identical in
  `BOARD.md` and the sidecar — just a `reviewer` string.
- Workaround: recorded the reviewer honestly as a self-review with that fact
  stated in the name, rather than inventing a second identity.
- Severity: medium — this may matter more than doctor/history for anyone
  running evidence-loop without a second real reviewer available.

## Protocol ambiguity

- (see S002)
