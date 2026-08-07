#!/usr/bin/env node
// Regenerate .claude-plugin/plugin.json's `skills` array from what is actually
// on disk.
//
// The manifest listing every skill by hand is the same shape of bug as keeping
// four copies of one file: it drifts silently, and nothing tells you. Adding a
// skill here means dropping a folder in and running this. --check fails CI if
// the manifest and the filesystem disagree.

import { readdirSync, readFileSync, writeFileSync, existsSync } from "fs";
import { dirname, join, relative } from "path";
import { fileURLToPath } from "url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const MANIFEST = join(ROOT, ".claude-plugin", "plugin.json");
const MARKETPLACE = join(ROOT, ".claude-plugin", "marketplace.json");
const LOCAL_PLUGIN = "magithar-skills";
const SKILLS_DIR = join(ROOT, "skills");

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

if (process.argv.includes("--check")) {
  if (same) { console.log(`plugin.json in sync (${found.length} skill(s))`); process.exit(0); }
  console.error("plugin.json is out of sync with skills/ on disk.");
  for (const p of found) if (!current.includes(p)) console.error("  missing from manifest: " + p);
  for (const p of current) if (!found.includes(p)) console.error("  in manifest but not on disk: " + p);
  console.error("Run: node scripts/sync-plugin.mjs");
  process.exit(1);
}

manifest.skills = found;
writeFileSync(MANIFEST, JSON.stringify(manifest, null, 2) + "\n");
console.log(`plugin.json updated: ${found.length} skill(s)`);
for (const p of found) console.log("  " + p);

syncMarketplace(found.length);

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
