# Magithar's Skills

Agent skills, installable as a Claude Code plugin. Each skill is a plain `SKILL.md`, so the same
file runs on Claude Code, Codex and Antigravity.

## Install

### Claude Code — add the marketplace once

```
/plugin marketplace add Magithar/skills
/plugin install skillmama@magithar
```

`/plugin marketplace update` pulls new and updated plugins later.

Plugins available:

| Plugin | What it is | Hosted in |
|---|---|---|
| `skillmama` | capability discovery: finds, scores and ranks libraries for your stack | [Magithar/SKILLmama](https://github.com/Magithar/SKILLmama) |
| `magithar-skills` | the skills under `skills/` in this repo, currently [`dep-egress`](skills/engineering/dep-egress/SKILL.md) | this repo |

A marketplace can list plugins that live in other repos, so SKILLmama keeps its own repository,
issues and release history while still being installable from here. Skills added directly to this
repo ship as a second plugin, `magithar-skills`, which is listed automatically once it contains at
least one skill.

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
.claude/skills/<name>/SKILL.md    symlink, so the skills work inside this repo too
scripts/sync-plugin.mjs           regenerates plugin.json and the symlinks from disk
tools/skill-land/               CLI: install a skill, then prove it landed
```

`tools/` sits outside `skills/`, so nothing in it is scanned or published to the plugin
manifest.

## tools/skill-land

Installs a skill to the directory the agent actually reads, then verifies the file landed and
exits non-zero if it didn't. Built because `npx skills add -g` reports success while writing
somewhere Codex and Antigravity never look.

```bash
npx skill-land Magithar/SKILLmama --for codex,antigravity
npx skill-land Magithar/SKILLmama --for codex --verify
```

Published as [`skill-land`](https://www.npmjs.com/package/skill-land) on npm.
Source and docs: [tools/skill-land/](tools/skill-land/README.md).

## Adding a skill

1. `mkdir -p skills/<category>/<name>` and write `SKILL.md` with `name:` and `description:`
   frontmatter.
2. `node scripts/sync-plugin.mjs`
3. Commit the skill, the updated `plugin.json`, and the new `.claude/skills` symlink.

`plugin.json` is generated, never hand-edited. A manifest maintained by hand drifts from the
filesystem silently, which is the same class of bug as keeping multiple copies of one file. CI
runs `sync-plugin.mjs --check` and fails if they disagree.

The script also maintains `.claude/skills/<name>/SKILL.md` as a symlink to each skill, so the
skills in this repo are usable *while working on this repo* — Claude Code only scans
`.claude/skills`, not `skills/<category>`. It creates, repairs and removes those links to match
disk, and refuses to overwrite anything under `.claude/skills` that isn't a symlink. Windows
contributors need `core.symlinks=true`; without it the links fail to create, which costs a local
slash command and nothing else.

The script also adds `magithar-skills` to `marketplace.json` on the first skill, and removes it
again if the last one goes. Publishing a plugin entry that installs nothing is worse than not
listing it: users get something that looks like it worked and did nothing.

Skills under `in-progress/` and `deprecated/` are ignored by the manifest, so you can park work in
the repo without shipping it.

## Related

- [SKILLmama](https://github.com/Magithar/SKILLmama) — capability discovery engine. Kept in its own
  repo because its published articles and skills.sh listing point there.

## License

MIT
