#!/usr/bin/env node
// Regenerate .claude-plugin/plugin.json's `skills` array, and the .claude/skills
// symlinks, from what is actually on disk.
//
// The manifest listing every skill by hand is the same shape of bug as keeping
// four copies of one file: it drifts silently, and nothing tells you. Adding a
// skill here means dropping a folder in and running this. --check fails CI if
// the manifest and the filesystem disagree.
//
// The symlinks exist so a skill in this repo is usable *while working on this
// repo*: Claude Code only scans .claude/skills, not skills/<category>. Doing it
// by hand is the same drift bug one level down — a missing link is invisible,
// the skill just silently isn't there.

import {
  lstatSync, mkdirSync, readdirSync, readFileSync, readlinkSync,
  rmSync, symlinkSync, writeFileSync, existsSync,
} from "fs";
import { dirname, join, relative } from "path";
import { fileURLToPath } from "url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const MANIFEST = join(ROOT, ".claude-plugin", "plugin.json");
const MARKETPLACE = join(ROOT, ".claude-plugin", "marketplace.json");
const LOCAL_PLUGIN = "magithar-skills";
const SKILLS_DIR = join(ROOT, "skills");
const LINK_DIR = join(ROOT, ".claude", "skills");

// A skill is any directory containing a SKILL.md. Categories are just folders;
// `deprecated` and `in-progress` are excluded from the published manifest.
const EXCLUDE = new Set(["deprecated", "in-progress"]);

function findSkills() {
  const out = [];
  for (const cat of readdirSync(SKILLS_DIR, { withFileTypes: true })) {
    if (!cat.isDirectory() || EXCLUDE.has(cat.name)) continue;
    for (const s of readdirSync(join(SKILLS_DIR, cat.name), { withFileTypes: true })) {
      if (!s.isDirectory()) continue;
      const skillMd = join(SKILLS_DIR, cat.name, s.name, "SKILL.md");
      if (existsSync(skillMd)) out.push("./" + relative(ROOT, dirname(skillMd)));
    }
  }
  return out.sort();
}

function frontmatterName(p) {
  const m = readFileSync(p, "utf-8").match(/^---\n([\s\S]*?)\n---/);
  const line = m && m[1].split("\n").find((l) => l.startsWith("name:"));
  return line ? line.slice(5).trim() : null;
}

// .claude/skills/<name>/SKILL.md -> ../../../skills/<category>/<name>/SKILL.md
// Keyed on the skill's directory name, which is what the manifest already uses.
function desiredLinks(paths) {
  return paths.map((p) => {
    const name = p.split("/").pop();
    const linkPath = join(LINK_DIR, name, "SKILL.md");
    return { name, linkPath, target: relative(dirname(linkPath), join(ROOT, p, "SKILL.md")) };
  });
}

// Only ever touch entries that are symlinks. A real file or directory under
// .claude/skills is someone's hand-written skill; report the collision rather
// than deleting their work to make room for a generated link.
function linkDrift(want) {
  const missing = [], wrong = [], stale = [], foreign = [];
  for (const w of want) {
    let st;
    try { st = lstatSync(w.linkPath); } catch { missing.push(w); continue; }
    if (!st.isSymbolicLink()) foreign.push(w);
    else if (readlinkSync(w.linkPath) !== w.target) wrong.push(w);
  }
  const keep = new Set(want.map((w) => w.name));
  let entries;
  try { entries = readdirSync(LINK_DIR, { withFileTypes: true }); } catch { return { missing, wrong, stale, foreign }; }
  for (const e of entries) {
    if (keep.has(e.name)) continue;
    const linkPath = join(LINK_DIR, e.name, "SKILL.md");
    let st;
    try { st = lstatSync(linkPath); } catch { continue; }
    if (st.isSymbolicLink()) stale.push({ name: e.name, linkPath });
  }
  return { missing, wrong, stale, foreign };
}

function syncLinks(want) {
  const { missing, wrong, stale, foreign } = linkDrift(want);
  for (const f of foreign) {
    console.error(`  .claude/skills/${f.name}/SKILL.md exists and is not a symlink; leaving it alone`);
  }
  for (const w of [...missing, ...wrong]) {
    try {
      mkdirSync(dirname(w.linkPath), { recursive: true });
      rmSync(w.linkPath, { force: true });
      symlinkSync(w.target, w.linkPath);
      console.log(`  linked .claude/skills/${w.name}/SKILL.md -> ${w.target}`);
    } catch (e) {
      // Windows refuses symlinks without developer mode or elevation. That
      // costs the contributor a local slash command, not a broken repo.
      console.error(`  could not link ${w.name}: ${e.code || e.message}`);
    }
  }
  for (const s of stale) {
    rmSync(dirname(s.linkPath), { recursive: true, force: true });
    console.log(`  unlinked .claude/skills/${s.name} (no longer in skills/)`);
  }
  return foreign.length;
}

const found = findSkills();
const manifest = JSON.parse(readFileSync(MANIFEST, "utf-8"));
const current = manifest.skills || [];

// Every skill needs a frontmatter name or agents cannot address it.
const nameless = found.filter((p) => !frontmatterName(join(ROOT, p, "SKILL.md")));
if (nameless.length) {
  console.error("SKILL.md missing a frontmatter `name:`:");
  for (const p of nameless) console.error("  " + p);
  process.exit(1);
}

const same = current.length === found.length && current.every((p, i) => p === found[i]);
const want = desiredLinks(found);

if (process.argv.includes("--check")) {
  const { missing, wrong, stale, foreign } = linkDrift(want);
  const linksOk = !missing.length && !wrong.length && !stale.length && !foreign.length;
  if (same && linksOk) {
    console.log(`plugin.json and .claude/skills in sync (${found.length} skill(s))`);
    process.exit(0);
  }
  if (!same) {
    console.error("plugin.json is out of sync with skills/ on disk.");
    for (const p of found) if (!current.includes(p)) console.error("  missing from manifest: " + p);
    for (const p of current) if (!found.includes(p)) console.error("  in manifest but not on disk: " + p);
  }
  if (!linksOk) {
    console.error(".claude/skills is out of sync with skills/ on disk.");
    for (const w of missing) console.error("  missing link: .claude/skills/" + w.name);
    for (const w of wrong) console.error("  link points elsewhere: .claude/skills/" + w.name);
    for (const s of stale) console.error("  link with no skill behind it: .claude/skills/" + s.name);
    for (const f of foreign) console.error("  not a symlink, cannot manage: .claude/skills/" + f.name);
  }
  console.error("Run: node scripts/sync-plugin.mjs");
  process.exit(1);
}

manifest.skills = found;
writeFileSync(MANIFEST, JSON.stringify(manifest, null, 2) + "\n");
console.log(`plugin.json updated: ${found.length} skill(s)`);
for (const p of found) console.log("  " + p);

const blocked = syncLinks(want);
syncMarketplace(found.length);

// A collision left in place means --check will keep failing. Say so loudly
// rather than exiting 0 on a state the next CI run rejects.
if (blocked) process.exit(1);

// The local plugin is only listed in the marketplace once it actually contains
// a skill. Publishing an entry that installs to nothing is worse than not
// listing it: users get a plugin that appears to work and does nothing.
function syncMarketplace(skillCount) {
  const mp = JSON.parse(readFileSync(MARKETPLACE, "utf-8"));
  const has = mp.plugins.some((p) => p.name === LOCAL_PLUGIN);
  if (skillCount > 0 && !has) {
    mp.plugins.push({
      name: LOCAL_PLUGIN,
      source: "./",
      description: manifest.description,
      category: "engineering",
      keywords: manifest.keywords || [],
    });
    writeFileSync(MARKETPLACE, JSON.stringify(mp, null, 2) + "\n");
    console.log(`marketplace.json: listed "${LOCAL_PLUGIN}" (now has ${skillCount} skill(s))`);
  } else if (skillCount === 0 && has) {
    mp.plugins = mp.plugins.filter((p) => p.name !== LOCAL_PLUGIN);
    writeFileSync(MARKETPLACE, JSON.stringify(mp, null, 2) + "\n");
    console.log(`marketplace.json: unlisted "${LOCAL_PLUGIN}" (0 skills — nothing to install)`);
  }
}
