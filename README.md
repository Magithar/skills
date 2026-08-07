# Magithar's Skills

Agent skills, installable as a Claude Code plugin. Each skill is a plain `SKILL.md`, so the same
file runs on Claude Code, Codex and Antigravity.

## Install

### Claude Code — the whole set, once

```
/plugin marketplace add Magithar/skills
/plugin install magithar-skills@magithar
```

New skills arrive with plugin updates. Nothing to re-install per skill.

### One skill, any agent

Skills are plain files, so a copy is a valid install. The directory differs per agent:

| Agent | Global skills directory |
|---|---|
| Claude Code | `~/.claude/skills/<name>/SKILL.md` |
| OpenAI Codex | `~/.codex/skills/<name>/SKILL.md` |
| Antigravity | `~/.gemini/config/skills/<name>/SKILL.md` |

```bash
mkdir -p ~/.codex/skills/<name>
curl -sL https://raw.githubusercontent.com/Magithar/skills/main/skills/<category>/<name>/SKILL.md \
  -o ~/.codex/skills/<name>/SKILL.md
```

Restart Antigravity after installing; it reads skills at startup.

> `npx skills add ... -g` currently writes to `~/.agents/skills/` for Codex and Antigravity, which
> neither agent reads, and still exits 0. Verified against `skills@1.5.22` on 2026-08-08. Tracked
> upstream in [vercel-labs/skills#1060](https://github.com/vercel-labs/skills/issues/1060), fix
> pending in [PR #1483](https://github.com/vercel-labs/skills/pull/1483).

## Layout

```
skills/
  engineering/<name>/SKILL.md
  productivity/<name>/SKILL.md
  in-progress/<name>/SKILL.md     not published to the plugin manifest
  deprecated/<name>/SKILL.md      not published to the plugin manifest
.claude-plugin/
  marketplace.json                makes this repo an installable marketplace
  plugin.json                     lists every published skill
scripts/sync-plugin.mjs           regenerates plugin.json from disk
tools/skill-land/               CLI: install a skill, then prove it landed
```

`tools/` sits outside `skills/`, so nothing in it is scanned or published to the plugin
manifest.

## tools/skill-land

Installs a skill to the directory the agent actually reads, then verifies the file landed and
exits non-zero if it didn't. Built because `npx skills add -g` reports success while writing
somewhere Codex and Antigravity never look.

```bash
node tools/skill-land/skill-land.mjs Magithar/SKILLmama --for codex,antigravity
node tools/skill-land/skill-land.mjs Magithar/SKILLmama --for codex --verify
```

See [tools/skill-land/README.md](tools/skill-land/README.md).

## Adding a skill

1. `mkdir -p skills/<category>/<name>` and write `SKILL.md` with `name:` and `description:`
   frontmatter.
2. `node scripts/sync-plugin.mjs`
3. Commit both the skill and the updated `plugin.json`.

`plugin.json` is generated, never hand-edited. A manifest maintained by hand drifts from the
filesystem silently, which is the same class of bug as keeping multiple copies of one file. CI
runs `sync-plugin.mjs --check` and fails if they disagree.

Skills under `in-progress/` and `deprecated/` are ignored by the manifest, so you can park work in
the repo without shipping it.

## Related

- [SKILLmama](https://github.com/Magithar/SKILLmama) — capability discovery engine. Kept in its own
  repo because its published articles and skills.sh listing point there.

## License

MIT
