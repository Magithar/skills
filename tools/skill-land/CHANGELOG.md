# Changelog

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
