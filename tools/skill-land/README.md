# skill-land

Install an agent skill to the directory the agent **actually reads**, then verify it landed.
Exits non-zero if it didn't.

```bash
npx skill-land Magithar/SKILLmama --for codex,antigravity
npx skill-land Magithar/SKILLmama --for codex --verify   # audit an existing install
```

[![npm](https://img.shields.io/npm/v/skill-land)](https://www.npmjs.com/package/skill-land)

## Why this exists

`npx skills add -a <agent> -g` reports success while writing to the wrong directory
for 13 agents, including Codex, Antigravity, Cursor, GitHub Copilot, Gemini CLI and
opencode. Reproduced against `skills@1.5.22` (latest) on 2026-08-08:

```
$ npx skills add Magithar/SKILLmama -a codex -g -y
  copy → Codex
  ✓ SKILLmama (copied)
    → ~/.agents/skills/skillmama
  Done!
$ echo $?
0
$ ls ~/.codex/skills/
                        # empty. Codex never sees it.
```

The correct path is in the CLI's own registry and never read. `isUniversalAgent()`
(`dist/cli.mjs:1678`) classifies any agent whose *project* directory is
`.agents/skills` as "universal", and `getAgentBaseDir()` then routes it to the
canonical store, discarding `globalSkillsDir`:

```js
function isUniversalAgent(type) {
	return agents[type].skillsDir === ".agents/skills";
}
```

Every failure of this kind exits 0. Nothing in the ecosystem checks the install took.

A second, separate bug: `-a claude-code,antigravity` is rejected as "Invalid agents"
while both names appear in the valid list it prints.

## Usage

```bash
npx skill-land <source> --for <agent> [options]
```

`<source>` is `owner/repo`, a GitHub URL, a raw `.md` URL, or a local path.

| Option | Effect |
|---|---|
| `--for <agent>` | target agent; repeatable or comma-separated |
| `--skill <name>` | which skill, when the source contains several |
| `--all` | every skill in the source, not just one |
| `--verbose` | per-skill detail when `--all` (default: one line each) |
| `--project` | install into `./` instead of the user-global directory |
| `--dry-run` | resolve and report, write nothing |
| `--verify` | check an existing install, write nothing |
| `--skip-security` | skip the SkillSpector scan (acknowledged, not silent) |
| `--strict` | refuse to install anything SkillSpector flags |
| `--list` | show known agents and their directories |

```bash
# install and verify
skill-land Magithar/SKILLmama --for codex,antigravity

# audit an install someone else did
skill-land Magithar/SKILLmama --for codex --verify

# audit every skill you have, however it was installed
skill-land ~/.agents/skills --for antigravity --verify --all
```

## Auditing a whole directory

`--all` takes every skill in the source instead of one. It is never the default:
[`selectSkill()`](skill-land.mjs) refuses to guess when a source holds several,
and `--all` is you saying you meant all of them.

The use it was built for is `--verify --all` — checking whether the skills you
already installed are where the agent actually reads, whatever put them there.
That question is unanswerable today even if your setup works, because nothing in
the ecosystem reads an install back:

```
$ skill-land ~/.agents/skills --for antigravity --verify --all

  source   /Users/you/.agents/skills
  skills   1 found
  impeccable                    FAIL  0/3
        x ~/.gemini/config/skills/impeccable/SKILL.md
          file exists: ~/.gemini/config/skills/impeccable/SKILL.md
        x ~/.gemini/skills/impeccable/SKILL.md
          content matches source: 10401 bytes sha abd76e36cf1d vs 10401 bytes sha a1ea82ce80f4
  1 skill, 3 failed
```

That middle line is why the check compares hashes and not sizes: both files are
10401 bytes and they are not the same file. A copy that drifted after install
looks identical to `ls`.

One line per skill, because a report nobody reads verifies nothing; `--verbose`
restores the full per-target output. Failing paths are always named — a summary
you have to re-run to act on is not a summary.

With `--strict`, a flagged skill is skipped and the rest still install; the run
exits non-zero and lists what it skipped. All-or-nothing across a directory would
let one flagged skill block every clean one.

## What "verified" means

An install is not done until read-back confirms it:

1. file exists at the target path
2. non-empty
3. byte-identical to the source
4. frontmatter `name` matches the source

Any failure prints the specific check that failed and **exits non-zero**. A write that
throws (permissions, read-only directory) is reported as a failed install, not a crash.

Tested against: truncated file, wrong skill in the right place, missing file,
unwritable destination, and a real `npx skills add` install (correctly reported FAIL).

## Antigravity

Antigravity loads skills at startup, so an install cannot be confirmed live. The
`agy` search path was stated by an Antigravity maintainer on 2026-09-05 in
[antigravity-cli#103](https://github.com/google-antigravity/antigravity-cli/issues/103#issuecomment-5547952239):

```
~/.gemini/config/skills            primary; independently confirmed by live testing
~/.gemini/skills                   fallback
~/.gemini/antigravity-cli/skills   fallback
```

`--for antigravity` writes to all three and says so. The `language_server` binary
contains `skillsPaths` and `skills_paths`, so this is a configurable list rather than a
single directory, and the maintainer statement covers the CLI — whether the IDE surface
reads the same list is **unverified**.

Checked against the shipped binaries (Antigravity 2026-07-30 build, inspected
2026-09-05). `language_server` contains **no** `.gemini/*/skills` literal at all:
`skills_paths` is a repeated protobuf field, so the client passes the list in, and
skills resolve to `skills/<name>/` under a *customization root*. Both roots we write
to are named in the shipped strings and config:

- `~/.gemini/config/` holds `config.json` and `skills/` — the global root
- `~/.gemini/antigravity-cli/` — "The CLI is configured via
  `~/.gemini/antigravity-cli/settings.json`" (`language_server`)

1.1.0 and earlier also wrote `~/.gemini/antigravity/skills`, on the reasoning that
Antigravity created that directory itself. That reasoning was wrong: `app.asar` defines
`~/.gemini/antigravity` as `IDE_OLD_DATA_DIR`, the **legacy IDE data directory** being
migrated to `~/.gemini/antigravity-ide`. It was never a skills root. No longer written.

### Live verification

Confirmed against **`agy` 1.1.27 on 2026-09-05**, not inferred. A uniquely-named
canary skill was planted in each candidate directory and `agy -p "/skills"` run from
a neutral workspace:

| directory | listed? |
|---|---|
| `~/.gemini/config/skills` | **yes** |
| `~/.gemini/skills` | **yes** |
| `~/.gemini/antigravity-cli/skills` | **yes** |
| `~/.agents/skills` | no |
| `~/.gemini/antigravity/skills` | no |

All three directories `skill-land` writes to are read. The two it does not write to
are not.

`~/.agents/skills` appears **only** when its parent is the active workspace
(`agy --add-dir $HOME`) — that is project-local behaviour, not a global search path,
which is exactly what [#103](https://github.com/google-antigravity/antigravity-cli/issues/103)
is asking to change. Confirmed still open at 1.1.27.

Two side findings: project-local `.agents/skills` is read only when the directory is
a **registered workspace** — being the current directory was not enough — and
`.antigravity/skills` and `.agy/skills` are not read at all.

Restart Antigravity fully after installing.

## Security scan

Skills run with full agent permissions, and this tool writes a file fetched from a URL into a
directory an agent auto-loads. So it scans before writing, delegating to
[NVIDIA/SkillSpector](https://github.com/NVIDIA/SkillSpector) rather than reimplementing it:

```bash
uv tool install git+https://github.com/NVIDIA/skillspector.git
```

### What always runs: disclosure

A skill is instructions an agent executes with full permissions, so before writing anything
`skill-land` shows what is actually in the file. No dependencies, so it runs for everyone:

```
contains
    3  shell commands        curl, rm, git
    1  network hosts         attacker.example.com
    5  credential refs       AWS_SECRET_ACCESS_KEY, ~/.ssh, id_rsa, ~/.aws
    2  destructive commands  rm -rf, git push --force
```

A skill with nothing notable says so:

```
contains  no shell commands, network calls or credential references
```

**This is disclosure, not a verdict.** It counts and names; it never scores and never prints
"safe". That distinction is deliberate. A built-in matcher that declared a skill safe would be
worse than no check, and the numbers below show why. Counting stays honest at any false-positive
rate because it makes no claim.

### What runs when available: SkillSpector

**It reports, it does not block.** That is a measured decision, not a shortcut.

Scanning 18 known-good skills already installed on a dev machine (2026-08-08, SkillSpector
v2.8.1, `--no-llm`):

| Verdict | Count |
|---|---|
| SAFE | 1 |
| CAUTION | 9 |
| **DO_NOT_INSTALL** | **8 (44%)** |

The failure mode is worth understanding, because it is not random. A skill that *documents* attack
patterns gets flagged for *containing* them: the static matcher has no negation handling, so a rule
saying "reject skills that bypass safety checks" reads identically to a skill saying "bypass safety
checks".

[SKILLmama](https://github.com/Magithar/SKILLmama) was a worked example. Its security checklist
scored `CRITICAL` until four DISCARD criteria were reworded to say the same thing in different
words; it now scores `7 / LOW / SAFE`. Its behaviour never changed, only its vocabulary. Adding an
explicit user-consent section was also tested and made the score **worse**, because describing a
safeguard trips the same patterns as describing the danger.

A gate that blocks 44% of legitimate skills does not make anyone safer; it teaches people to pass
the override flag reflexively. So findings are surfaced with file and line, and the decision stays
with the user. `--strict` blocks on a flag for anyone who wants that.

| SkillSpector verdict | Default | `--strict` |
|---|---|---|
| `SAFE` | installs | installs |
| `CAUTION` | installs, findings shown | installs |
| `DO_NOT_INSTALL` | installs, findings shown | **refuses, exit 1** |
| scanner error | `treated as NOT scanned` | same |
| not installed | `NOT SCANNED` + install hint | same |

### Baselines are reported, never applied

A skill repo can ship `.skillspector-baseline.yaml` to suppress findings. This installer scans
**without** it, deliberately: a baseline that travels with untrusted code would let a malicious
skill silence its own detections. SkillSpector does not auto-discover baselines either (verified:
a baseline in the scanned directory has no effect unless `--baseline` is passed), so honouring one
found in a fetched repo would be strictly less safe than the default.

Instead its presence is surfaced as a fact about the skill, which is itself a signal:

```
security SAFE  SAFE, LOW, score 7, 2 issue(s)
         NOTE this repo ships a baseline suppressing 2 finding(s).
              Scanned WITHOUT it: a baseline from an untrusted repo can hide real findings.
```

`--no-llm` is always passed. Without it SkillSpector requires an LLM provider and exits 2 with no
credentials, which would silently degrade this gate to a no-op for anyone without an API key. A
check that did not run must never read as a check that passed.

## Skill discovery

Walks up to 10 levels, skipping only `.git`, `node_modules`, `.venv` and
`__pycache__`, with a symlink-loop guard. Project-specific folder names are
deliberately not hardcoded: a repo whose skill lives in `assets/` is legal, and
skipping it produced a false "no SKILL.md found".

When a source contains more than one skill the tool **refuses to guess**. It lists
what it found and exits non-zero until you pass `--skill <name>`:

```
$ skill-land product-on-purpose/pm-skills --for codex
  error  78 skills found; refusing to guess.
         Pick one with --skill <name>:
           define-hypothesis    ./skills/define-hypothesis/SKILL.md
           ...
```

## Releasing

Published from CI by [trusted publishing](https://docs.npmjs.com/trusted-publishers), so there is
no `NPM_TOKEN` in this repo and the npm account keeps 2FA on write actions. npm mints a
short-lived token from the GitHub identity of `.github/workflows/publish-skill-land.yml`.

```bash
# bump version in package.json, commit, then:
git tag skill-land-v1.0.2
git push origin skill-land-v1.0.2
```

CI runs the tests, refuses to publish a version that already exists, checks the tag matches
`package.json`, and publishes. The tag is package-scoped so other tools in this repo can release
independently.

## Adding an agent

Edit `agents.json`. `broken: true` marks agents where the upstream CLI misroutes a
global install, which is what drives the warning note.

## Scope

Install and verify only. No registry, no publishing, no per-platform content
transforms. One `SKILL.md` works across all of these agents; the failure mode here is
a wrong-path bug, not a content-format problem.
