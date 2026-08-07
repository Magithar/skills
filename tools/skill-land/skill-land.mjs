#!/usr/bin/env node
// skill-land — install an agent skill and verify it actually landed.
//
// Exists because `npx skills add -a <agent> -g` reports success while writing to
// ~/.agents/skills for 13 agents (Codex, Antigravity, Cursor, Copilot, ...). The
// correct path is in its registry and never read: isUniversalAgent() (cli.mjs:1678)
// classifies any agent whose PROJECT dir is .agents/skills as "universal" and
// discards its globalSkillsDir. Every such failure exits 0.
//
// This tool owns the path table and refuses to exit 0 on an unverified install.

import { execFileSync } from "child_process";
import { createHash } from "crypto";
import {
  cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync,
  readFileSync, realpathSync, rmSync, statSync,
} from "fs";
import { homedir, tmpdir } from "os";
import { basename, dirname, join, resolve } from "path";
import { fileURLToPath } from "url";

const HERE = dirname(fileURLToPath(import.meta.url));
const { agents: AGENTS } = JSON.parse(readFileSync(join(HERE, "agents.json"), "utf-8"));

const sha256 = (buf) => createHash("sha256").update(buf).digest("hex");
const expand = (p) => (p.startsWith("~/") ? join(homedir(), p.slice(2)) : p);
const die = (msg) => { console.error(`\n  error  ${msg}\n`); process.exit(1); };

// ---------------------------------------------------------------- source

// Accepts: owner/repo | https://github.com/owner/repo | raw .md URL | local path
function resolveSource(input) {
  if (existsSync(input) && statSync(input).isDirectory())
    return { kind: "dir", dir: resolve(input), label: resolve(input) };

  if (/^https?:\/\/.*\.md(\?.*)?$/i.test(input))
    return { kind: "raw", url: input, label: input };

  const slug = input.replace(/^https?:\/\/github\.com\//i, "").replace(/\.git$/, "").replace(/\/$/, "");
  if (!/^[\w.-]+\/[\w.-]+$/.test(slug))
    die(`cannot parse source "${input}"\n         expected owner/repo, a github URL, a raw .md URL, or a local path`);
  return { kind: "repo", url: `https://github.com/${slug}.git`, label: slug };
}

function fetchSource(src, tmp) {
  if (src.kind === "dir") return src.dir;
  if (src.kind === "raw") {
    const dest = join(tmp, "SKILL.md");
    execFileSync("curl", ["-fsSL", src.url, "-o", dest], { stdio: "pipe" });
    return tmp;
  }
  execFileSync("git", ["clone", "--depth", "1", "--quiet", src.url, tmp], { stdio: "pipe" });
  return tmp;
}

// ---------------------------------------------------------------- discovery

// Only skip things that are never a skill anywhere. Project-specific folder
// names must not be hardcoded here: a repo whose skill lives in assets/ is
// perfectly legal, and skipping it produced a false "no SKILL.md found".
const NEVER_SKILLS = new Set([".git", "node_modules", ".venv", "__pycache__"]);
const MAX_DEPTH = 10;

function findSkills(root) {
  const hits = [];
  const seen = new Set();
  (function walk(dir, depth) {
    if (depth > MAX_DEPTH) return;
    let real;
    try { real = statSync(dir).ino + ":" + statSync(dir).dev; } catch { return; }
    if (seen.has(real)) return; // symlink loop guard
    seen.add(real);
    let entries;
    try { entries = readdirSync(dir, { withFileTypes: true }); } catch { return; }
    for (const e of entries) {
      const p = join(dir, e.name);
      if (e.isDirectory() && !NEVER_SKILLS.has(e.name)) { walk(p, depth + 1); continue; }
      // isFile() is false for symlinks, and the skills CLI symlinks by default,
      // so a symlinked SKILL.md must still count. Dedupe by real path below.
      if (e.name !== "SKILL.md") continue;
      if (e.isFile() || e.isSymbolicLink()) hits.push(p);
    }
  })(root, 0);

  // A repo may symlink its own skill (e.g. .claude/skills → skillmama/). Those
  // are the same skill, not two, so collapse by resolved target.
  const byReal = new Map();
  for (const p of hits) {
    let key; try { key = realpathSync(p); } catch { key = p; }
    const prev = byReal.get(key);
    if (!prev || p.split("/").length < prev.split("/").length) byReal.set(key, p);
  }
  return [...byReal.values()].sort(
    (a, b) => a.split("/").length - b.split("/").length || a.localeCompare(b));
}

// Never silently pick one skill out of many. A repo with 78 skills must not
// install an arbitrary one and report success.
function selectSkill(root, wanted) {
  const hits = findSkills(root);
  if (!hits.length)
    die(`no SKILL.md found under ${root}\n         searched ${MAX_DEPTH} levels deep`);

  const named = hits.map((p) => ({
    path: p,
    name: parseFrontmatter(readFileSync(p, "utf-8")).name || basename(dirname(p)),
  }));

  if (wanted) {
    const w = wanted.toLowerCase();
    const match = named.filter((n) => n.name.toLowerCase() === w);
    if (!match.length)
      die(`no skill named "${wanted}" in ${root}\n         available: ${named.map((n) => n.name).join(", ")}`);
    if (match.length > 1)
      die(`"${wanted}" is ambiguous, ${match.length} matches:\n         ${match.map((m) => m.path.replace(root, ".")).join("\n         ")}`);
    return match[0].path;
  }

  if (named.length > 1) {
    const shown = named.slice(0, 15).map((n) => `           ${n.name.padEnd(28)} ${n.path.replace(root, ".")}`);
    die(
      `${named.length} skills found in ${root}; refusing to guess.\n` +
      `         Pick one with --skill <name>:\n` +
      shown.join("\n") +
      (named.length > 15 ? `\n           ... and ${named.length - 15} more` : "")
    );
  }
  return named[0].path;
}

function parseFrontmatter(text) {
  const m = text.match(/^---\n([\s\S]*?)\n---\n/);
  if (!m) return {};
  const fm = {};
  for (const line of m[1].split("\n")) {
    const i = line.indexOf(":");
    if (i > 0) fm[line.slice(0, i).trim()] = line.slice(i + 1).trim();
  }
  return fm;
}

// ---------------------------------------------------------------- security

// Skills run with full agent permissions, and this tool writes a file fetched
// from a URL into a directory an agent auto-loads. So scan before writing.
//
// Delegates to NVIDIA/SkillSpector rather than reimplementing its pattern set.
// Exit codes per its docs: 0 = SAFE or CAUTION, 1 = DO_NOT_INSTALL, 2 = error.
//
// Borrowed from SKILLmama's own Phase 3.5 lesson: never let a check that did
// not run read as a check that passed. When the scanner is absent we say so
// loudly and mark the install unscanned, rather than silently continuing.
// --no-llm is not optional. Without it SkillSpector needs an LLM provider and
// exits 2 with no credentials, which would silently degrade this gate to a
// no-op for anyone without an API key.
function securityScan(skillPath) {
  let out = "", code = 0;
  try {
    out = execFileSync("skillspector",
      ["scan", skillPath, "--no-llm", "--format", "json"],
      { encoding: "utf-8", stdio: ["ignore", "pipe", "pipe"], maxBuffer: 32 * 1024 * 1024 });
  } catch (e) {
    if (e.code === "ENOENT") return { status: "unavailable" };
    code = typeof e.status === "number" ? e.status : 2;
    out = e.stdout || "";
  }

  let ra = null, issues = [];
  try {
    const j = JSON.parse(out);
    ra = j.risk_assessment || null;          // { score, severity, recommendation }
    issues = Array.isArray(j.issues) ? j.issues : [];
  } catch { /* fall through */ }

  if (!ra) return { status: "error", detail: `skillspector exited ${code}, no parsable report` };

  // A repo can ship .skillspector-baseline.yaml to suppress findings. We scan
  // WITHOUT it deliberately: honouring a baseline that travels with untrusted
  // code would let a malicious skill silence its own detections. Instead the
  // baseline is surfaced as a fact about the skill, which is itself a signal.
  let baseline = null;
  for (const dir of [dirname(skillPath), join(dirname(skillPath), "..")]) {
    const b = join(dir, ".skillspector-baseline.yaml");
    if (existsSync(b)) {
      const n = (readFileSync(b, "utf-8").match(/^\s*-\s*hash:/gm) || []).length;
      baseline = { path: b, count: n };
      break;
    }
  }

  // Findings are reported, never used to block by default. Measured on this
  // machine 2026-08-08: 8 of 18 known-good installed skills scored
  // DO_NOT_INSTALL (44%), and SKILLmama's own SKILL.md scored 100/CRITICAL
  // because its Phase 3.7 DISCARD rules literally contain the phrase
  // "instructions to bypass safety checks". A gate that blocks half of all
  // legitimate skills is worse than no gate: people learn to pass --force.
  const top = [...issues]
    .sort((a, b) => (b.confidence || 0) - (a.confidence || 0))
    .slice(0, 3)
    .map((i) => `${i.id} ${i.severity} ${i.location?.file || "?"}:${i.location?.start_line ?? "?"}`);

  return {
    status: ra.recommendation === "DO_NOT_INSTALL" ? "flagged"
          : ra.recommendation === "CAUTION" ? "caution" : "pass",
    detail: `${ra.recommendation}, ${ra.severity}, score ${ra.score}, ${issues.length} issue(s)`,
    top, baseline,
  };
}

// ---------------------------------------------------------------- verify

// An install is not done until read-back confirms it. This is the whole point.
function verify(target, expected) {
  const checks = [];
  const ok = (name, pass, detail) => { checks.push({ name, pass, detail }); return pass; };

  if (!ok("file exists", existsSync(target), target)) return checks;

  // Compare the raw bytes by hash. An earlier version compared
  // readFileSync(...,"utf-8").length, which is UTF-16 code units, not bytes:
  // it read 25266 for a 25546-byte file and would pass two different files
  // that happened to share a character count.
  const raw = readFileSync(target);
  ok("non-empty", raw.length > 0, `${raw.length} bytes`);
  const gotHash = sha256(raw);
  ok("content matches source", gotHash === expected.hash,
     `${raw.length} bytes sha ${gotHash.slice(0, 12)} vs ${expected.bytes} bytes sha ${expected.hash.slice(0, 12)}`);

  const fm = parseFrontmatter(raw.toString("utf-8"));
  if (expected.name) {
    ok("frontmatter name matches", fm.name === expected.name,
       `found ${fm.name ? `"${fm.name}"` : "none"}, expected "${expected.name}"`);
  } else {
    ok("parses", true, "no frontmatter in source, skipped name check");
  }
  return checks;
}

// ---------------------------------------------------------------- main

const argv = process.argv.slice(2);
if (!argv.length || argv.includes("-h") || argv.includes("--help")) {
  console.log(`
  skill-land <source> --for <agent> [options]

    <source>     owner/repo | github URL | raw .md URL | local path

  Options
    --for <agent>   target agent (repeatable, or comma-separated)
    --skill <name>  which skill, when the source contains several
    --project       install into ./ instead of the user-global dir
    --dry-run       resolve and report, write nothing
    --verify        check an existing install, write nothing
    --skip-security skip the SkillSpector scan (acknowledged, not silent)
    --strict        refuse to install anything SkillSpector flags
    --list          show known agents and their directories

  Verifies the file landed where the agent actually reads. Exits non-zero if not.
`);
  process.exit(0);
}

if (argv.includes("--list")) {
  console.log("\n  agent             directory                              upstream");
  console.log("  " + "-".repeat(70));
  for (const [k, a] of Object.entries(AGENTS)) {
    console.log(`  ${k.padEnd(17)} ${a.global.padEnd(38)} ${a.broken ? "BROKEN" : "ok"}`);
  }
  console.log();
  process.exit(0);
}

const flag = (n) => argv.includes(n);
const values = (n) => argv.reduce((acc, a, i) =>
  a === n && argv[i + 1] ? acc.concat(argv[i + 1].split(",")) : acc, []);

// Flags that consume the next argument; their values are not the source.
const VALUE_FLAGS = new Set(["--for", "--skill"]);
const source = argv.find((a, i) => !a.startsWith("--") && !VALUE_FLAGS.has(argv[i - 1]));
if (!source) die("no source given");

const targets = values("--for").map((s) => s.trim()).filter(Boolean);
if (!targets.length) die(`no --for <agent> given\n         known: ${Object.keys(AGENTS).join(", ")}`);
for (const t of targets) if (!AGENTS[t]) die(`unknown agent "${t}"\n         known: ${Object.keys(AGENTS).join(", ")}`);

const isProject = flag("--project");
const dryRun = flag("--dry-run");
const verifyOnly = flag("--verify");
const skipSecurity = flag("--skip-security");
const strict = flag("--strict");

const src = resolveSource(source);
const tmp = mkdtempSync(join(tmpdir(), "skill-install-"));
let failed = 0;

try {
  const root = fetchSource(src, tmp);
  const skillPath = selectSkill(root, values("--skill")[0]);
  const body = readFileSync(skillPath, "utf-8");
  const fm = parseFrontmatter(body);
  const skillName = (fm.name || "skill").toLowerCase().replace(/[^a-z0-9._-]+/g, "-");
  const rawBody = readFileSync(skillPath);
  const expected = { bytes: rawBody.length, hash: sha256(rawBody), name: fm.name };

  // Scan before any write. A blocked skill must never reach disk.
  const scan = skipSecurity ? { status: "skipped" } : securityScan(skillPath);
  const findings = (scan.top || []).length
    ? "\n" + scan.top.map((t) => `           ${t}`).join("\n") : "";
  // Reported, never applied. See securityScan().
  const baselineNote = scan.baseline
    ? `\n           NOTE this repo ships a baseline suppressing ${scan.baseline.count} finding(s).`
    + `\n                Scanned WITHOUT it: a baseline from an untrusted repo can hide real findings.`
    : "";
  const SCAN_LINE = {
    pass:        () => `  security SAFE  ${scan.detail}${baselineNote}`,
    caution:     () => `  security CAUTION  ${scan.detail}${findings}${baselineNote}`,
    flagged:     () => `  security FLAGGED  ${scan.detail}${findings}\n           SkillSpector flags ~44% of known-good skills; review, don't just trust it.\n           --strict to refuse installing flagged skills.${baselineNote}`,
    error:       () => `  security ERROR  ${scan.detail}  — treated as NOT scanned`,
    unavailable: () => `  security NOT SCANNED  skillspector not on PATH\n           uv tool install git+https://github.com/NVIDIA/skillspector.git`,
    skipped:     () => `  security NOT SCANNED  --skip-security given`,
  };

  console.log(`\n  source   ${src.label}`);
  console.log(`  skill    ${fm.name || "(no frontmatter name)"}  [${rawBody.length} bytes, sha ${expected.hash.slice(0,12)}]`);
  console.log(`  from     ${skillPath.replace(root, ".")}`);
  if (!verifyOnly) console.log(SCAN_LINE[scan.status]());
  console.log("");

  // Refuse to write a skill the scanner flagged. --force is an explicit,
  // logged override, never a silent one.
  if (!verifyOnly && strict && scan.status === "flagged") {
    die(`--strict: SkillSpector reports ${scan.detail}\n         Review the findings above. Drop --strict to install anyway.`);
  }

  for (const agent of targets) {
    const a = AGENTS[agent];
    const bases = isProject
      ? [resolve(a.project)]
      : [a.global, ...(a.alsoWrite || [])].map(expand);

    console.log(`  ${a.display}`);

    for (const base of bases) {
      const destDir = join(base, skillName);
      const target = join(destDir, a.file);

      if (dryRun) { console.log(`    would write  ${target}`); continue; }

      // A write that throws is a failed install, not a crash. Report it the
      // same way as a failed read-back so one exit path covers both.
      let writeErr = null;
      if (!verifyOnly) {
        try {
          mkdirSync(destDir, { recursive: true });
          cpSync(skillPath, target);
        } catch (e) {
          writeErr = `${e.code || "error"}: ${e.message.split("\n")[0]}`;
        }
      }

      const checks = writeErr
        ? [{ name: "write", pass: false, detail: writeErr }]
        : verify(target, expected);
      const passed = checks.every((c) => c.pass);
      if (!passed) failed++;

      console.log(`    ${passed ? "OK  " : "FAIL"}  ${target}${verifyOnly ? "  (verify only)" : ""}`);
      for (const c of checks) {
        if (!c.pass) console.log(`            x ${c.name}: ${c.detail}`);
      }
    }

    if (!dryRun && a.restart) console.log(`    note  restart ${a.display} fully; skills load at startup`);
    if (a.broken) console.log(`    note  \`npx skills add -a ${agent} -g\` writes to ~/.agents/skills instead`);
    console.log();
  }
} finally {
  rmSync(tmp, { recursive: true, force: true });
}

if (failed) {
  console.error(`  ${failed} install(s) failed verification\n`);
  process.exit(1);
}
if (!dryRun) console.log(`  ${verifyOnly ? "verified existing install" : "verified"}\n`);
