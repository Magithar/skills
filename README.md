<h1 align="center">Magithar's Skills</h1>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"/>
  <a href="https://github.com/Magithar/skills/actions/workflows/check.yml"><img src="https://github.com/Magithar/skills/actions/workflows/check.yml/badge.svg" alt="CI"/></a>
  <a href="https://www.npmjs.com/package/skill-land"><img src="https://img.shields.io/npm/v/skill-land?label=skill-land" alt="skill-land on npm"/></a>
</p>

<p align="center">
  Agent skills that do one job each and say plainly what they do not do. Every skill is a plain <code>SKILL.md</code> with no runtime, no scripts and no dependencies, so the same file works on every agent that reads skills. Install the whole set as a Claude Code plugin, or copy a single file.<br/><br/>
  Works with <a href="#claude-code">Claude Code</a>, <a href="#one-skill-any-agent">OpenAI Codex</a>, and <a href="#one-skill-any-agent">Antigravity</a>.
</p>

---

<p align="center">
  <a href="#skills">Skills</a> • <a href="#install">Install</a> • <a href="#layout">Layout</a> • <a href="#toolsskill-land">skill-land</a> • <a href="#adding-a-skill">Adding a Skill</a> • <a href="#design-rules">Design Rules</a>
</p>

---

## Skills

| Skill | What it answers | Where it lives |
|---|---|---|
| [`dep-egress`](skills/engineering/dep-egress/SKILL.md) | What does this dependency send off the machine, to whom, is it disclosed, and can it be turned off? | this repo |
| [`skillmama`](https://github.com/Magithar/SKILLmama) | Which library, SDK or tool should I use for my stack, and is it safe? | [Magithar/SKILLmama](https://github.com/Magithar/SKILLmama) |

The two compose. SKILLmama ranks candidates on compatibility, popularity, maintenance and
simplicity. `dep-egress` runs on the winner and answers the question ranking says nothing about.

A marketplace can list plugins that live in other repos, so SKILLmama keeps its own repository,
issues and release history while still being installable from here.

---

## Install

### Claude Code

Add the marketplace once, then install what you want:

```
/plugin marketplace add Magithar/skills
/plugin install magithar-skills@magithar
/plugin install skillmama@magithar
```

`/plugin marketplace update` pulls new and updated plugins later.

| Plugin | Contains |
|---|---|
| `magithar-skills` | every skill under `skills/` in this repo |
| `skillmama` | SKILLmama, hosted in its own repo |

### One skill, any agent

Skills are plain files, so a copy is a valid install. Only the directory differs:

| Agent | Global skills directory |
|---|---|
| Claude Code | `~/.claude/skills/<name>/SKILL.md` |
| OpenAI Codex | `~/.codex/skills/<name>/SKILL.md` |
| Antigravity | `~/.gemini/config/skills/<name>/SKILL.md` |

```bash
mkdir -p ~/.codex/skills/dep-egress
curl -sL https://raw.githubusercontent.com/Magithar/skills/main/skills/engineering/dep-egress/SKILL.md \
  -o ~/.codex/skills/dep-egress/SKILL.md
```

Restart Antigravity after installing; it reads skills at startup.

> `npx skills add ... -g` currently writes to `~/.agents/skills/` for Codex and Antigravity, which
> neither agent reads, and still exits 0. Verified against `skills@1.5.22` on 2026-08-08. Tracked
> upstream in [vercel-labs/skills#1060](https://github.com/vercel-labs/skills/issues/1060), fix
> pending in [PR #1483](https://github.com/vercel-labs/skills/pull/1483).

---

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
tools/skill-land/                 CLI: install a skill, then prove it landed
```

`tools/` sits outside `skills/`, so nothing in it is scanned or published to the plugin manifest.

---

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

---

## Adding a skill

1. `mkdir -p skills/<category>/<name>` and write `SKILL.md` with `name:` and `description:`
   frontmatter.
2. `node scripts/sync-plugin.mjs`
3. Commit the skill, the updated `plugin.json`, and the new `.claude/skills` symlink.

`plugin.json` is generated, never hand-edited. A manifest maintained by hand drifts from the
filesystem silently, which is the same class of bug as keeping multiple copies of one file. CI runs
`sync-plugin.mjs --check` and fails if they disagree.

The script also maintains `.claude/skills/<name>/SKILL.md` as a symlink to each skill, so the skills
in this repo are usable *while working on this repo*. Claude Code only scans `.claude/skills`, not
`skills/<category>`, so without the link a skill silently is not there. The script creates, repairs
and removes those links to match disk, and refuses to overwrite anything under `.claude/skills` that
is not a symlink. Windows contributors need `core.symlinks=true`; without it the links fail to
create, which costs a local slash command and nothing else.

It also adds `magithar-skills` to `marketplace.json` on the first skill, and removes it again if the
last one goes. Publishing a plugin entry that installs nothing is worse than not listing it: users
get something that looks like it worked and did nothing.

Skills under `in-progress/` and `deprecated/` are ignored by the manifest, so you can park work in
the repo without shipping it.

---

## Design rules

Every skill here follows the same four rules. They are what makes a skill trustworthy rather than
merely present in a directory.

**One question per skill.** A skill that answers everything gets loaded for everything and is
reliably good at nothing. Each `SKILL.md` opens with the single question it answers.

**Say what it is not for.** Every skill carries an explicit do-not-activate list naming the adjacent
jobs it will not do, and which kind of skill does them instead. Scope stated only as inclusion
always drifts outward.

**Evidence, or say you could not verify.** Findings cite a file and a line. A conclusion that cannot
be traced to something read is reported as unverified, and "I found nothing" is stated as a claim
about the search rather than about the thing searched.

**No secrets in output.** Skills that read real projects report locations and field names, never
values. A report that copies live data into itself has caused the problem it was asked to detect.

---

## Related

- [SKILLmama](https://github.com/Magithar/SKILLmama), capability discovery engine. Kept in its own
  repo because its published articles and skills.sh listing point there.
- [skill-land](https://www.npmjs.com/package/skill-land) on npm.

## License

MIT
