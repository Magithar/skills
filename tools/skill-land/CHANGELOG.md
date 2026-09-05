# Changelog

## 1.2.0

### Added
- **`--all`: every skill in the source, not just one.** The third arm of a decision that already
  existed — `selectSkill()` could refuse to guess between several skills or take one by name, but
  never act on more than one of what it had already found. Never the default.
- **`--verify --all` audits a whole directory**: are the skills you installed actually where the
  agent reads? Nothing in the ecosystem answers that today, including for setups that work.
  Run against a real `~/.agents/skills` it found a skill missing from two of Antigravity's three
  paths and, in the third, a file of **identical byte length and a different hash** — the drift
  case a size check cannot see.
- **`--verbose`** restores full per-target output; `--all` prints one line per skill by default,
  since a report nobody reads verifies nothing. Failing paths are named either way.
- Eight tests covering `--all`, including that one bad skill among many exits non-zero, names
  itself, and does not mask the skills that passed.

### Changed
- **`--strict` with `--all` skips the flagged skill and installs the rest**, exiting non-zero with
  the skipped list. All-or-nothing across a directory would let one flagged skill block every
  clean one.
- **`--verify` no longer runs the security scan.** It writes nothing, so scanning before a write
  that never happens was pure cost — 20 SkillSpector subprocesses to audit 20 installs.

### Fixed
- **`--for antigravity` wrote to a directory nothing is known to read.** It targeted
  `~/.gemini/antigravity/skills` as a fallback, on the strength of the skills CLI registry
  declaring it and Antigravity creating the directory itself. An Antigravity maintainer
  [stated the CLI's search path on 2026-09-05](https://github.com/google-antigravity/antigravity-cli/issues/103#issuecomment-5547952239)
  and that path is not in it. Fallbacks are now `~/.gemini/skills` and
  `~/.gemini/antigravity-cli/skills`; the primary `~/.gemini/config/skills` is unchanged and is
  the one the maintainer lists first.
- This mattered because `verify()` reads back the copy it just wrote, so a write to a path no
  agent reads passed every check — the exact failure mode this tool exists to catch, in its own
  registry. Existing `~/.gemini/antigravity/skills` directories from earlier versions are left
  alone; delete them by hand if you want them gone.
- **The original reasoning for that path was wrong, not just unlucky.** It was written because
  "Antigravity creates the directory itself". Inspecting the shipped app: `app.asar` defines
  `~/.gemini/antigravity` as `IDE_OLD_DATA_DIR`, the legacy IDE data directory being migrated to
  `~/.gemini/antigravity-ide`. It was never a skills root. The directory's existence was evidence
  of a migration, and we read it as evidence of a search path.

### Notes
- Both roots we now write to are corroborated by the shipped binaries (Antigravity 2026-07-30
  build): `language_server` documents `~/.gemini/antigravity-cli/settings.json` as the CLI's
  config, and skills resolve to `skills/<name>/` under a customization root. `~/.gemini/config/`
  holds `config.json` and `skills/`. `language_server` hardcodes no `.gemini/*/skills` path at all
  — `skills_paths` is a repeated protobuf field the client fills in.
- **Live-verified against `agy` 1.1.27** (2026-09-05), not inferred: a canary skill in each
  candidate directory, `agy -p "/skills"` from a neutral workspace. All three directories we write
  to are listed; `~/.agents/skills` and `~/.gemini/antigravity/skills` are not. `~/.agents/skills`
  shows up only with `--add-dir $HOME`, i.e. as a project-local dir — so
  [#103](https://github.com/google-antigravity/antigravity-cli/issues/103) is confirmed still open
  at 1.1.27.
- Side findings, not acted on: project-local `.agents/skills` is read only when the directory is a
  registered workspace (cwd alone was not enough), and `.antigravity/skills` / `.agy/skills` are
  not read at all.

## 1.1.0

### Added
- **Disclosure now always runs**, with or without a scanner. Before writing anything, `skill-land`
  counts and names the shell commands, network hosts, credential references and destructive
  commands in the file. On a malicious fixture it surfaces `attacker.example.com`, `~/.ssh`,
  `id_rsa`, `rm -rf` and `git push --force` with no scanner installed. A skill with nothing
  notable says so explicitly.
- **A test suite**: 17 tests driving the CLI as a subprocess, asserting exit codes and output
  rather than internal functions. Runs against a temp `HOME` and never touches real skill
  directories.
- **Release from CI** via [npm trusted publishing](https://docs.npmjs.com/trusted-publishers).
  No `NPM_TOKEN` anywhere and the npm account keeps 2FA on write actions.

### Notes
- Disclosure is **not a verdict**. It counts and names; it never scores and never prints "safe".
  A built-in matcher claiming safety would be worse than no check: SkillSpector's static mode
  calls 44% of known-good skills `DO_NOT_INSTALL`. Counting stays honest at any false-positive
  rate because it makes no claim. SkillSpector still does the judging when installed.

## 1.0.1

### Fixed
- The npm page stated as present fact that SKILLmama scored `100 / CRITICAL / DO_NOT_INSTALL`.
  True when written, false hours later once the underlying issue was fixed, and it sat publicly
  next to that project's name. Rewritten in past tense with the resolution included.

## 1.0.0

Initial release.

- Installs a skill to the directory the agent **actually reads**. `npx skills add -a <agent> -g`
  reports success while writing to `~/.agents/skills` for 13 agents, including Codex, Antigravity,
  Cursor, GitHub Copilot, Gemini CLI and opencode. The correct path is in its own registry and
  never read, because `isUniversalAgent()` keys off the *project* directory. Every such failure
  exits 0. Upstream: [#1060](https://github.com/vercel-labs/skills/issues/1060),
  [#1470](https://github.com/vercel-labs/skills/issues/1470), fix pending in
  [PR #1483](https://github.com/vercel-labs/skills/pull/1483).
- **Verifies the install landed**: file exists, non-empty, sha256 matches source, frontmatter name
  matches. Exits non-zero otherwise. `--verify` audits an existing install without writing.
- Refuses to guess when a source contains several skills.
- A repo-shipped `.skillspector-baseline.yaml` is reported, never applied. Honouring a baseline
  that travels with untrusted code would let a malicious skill suppress its own detections.
