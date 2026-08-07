#!/usr/bin/env node
// Regression suite for skill-land. No framework, no dependencies.
//
// Runs the real CLI as a subprocess and asserts on exit codes and output, so
// it tests what a user actually gets rather than internal functions.
//
// Writes only inside a temp HOME, never the caller's real skill directories.

import { execFileSync } from "child_process";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, existsSync, readFileSync } from "fs";
import { tmpdir } from "os";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const HERE = dirname(fileURLToPath(import.meta.url));
const CLI = join(HERE, "skill-land.mjs");
const SANDBOX = mkdtempSync(join(tmpdir(), "skill-land-test-"));
const HOME = join(SANDBOX, "home");
mkdirSync(HOME, { recursive: true });

let pass = 0, fail = 0;

function run(args, opts = {}) {
  try {
    const stdout = execFileSync(process.execPath, [CLI, ...args], {
      encoding: "utf-8",
      env: { ...process.env, HOME, PATH: process.env.PATH },
      stdio: ["ignore", "pipe", "pipe"],
      ...opts,
    });
    return { code: 0, out: stdout };
  } catch (e) {
    return { code: e.status ?? 1, out: (e.stdout || "") + (e.stderr || "") };
  }
}

function t(name, args, expectCode, expectMatch) {
  const r = run(args);
  const codeOk = r.code === expectCode;
  const matchOk = !expectMatch || expectMatch.test(r.out);
  if (codeOk && matchOk) { pass++; console.log(`  pass  ${name}`); return; }
  fail++;
  console.log(`  FAIL  ${name}`);
  if (!codeOk) console.log(`          exit ${r.code}, expected ${expectCode}`);
  if (!matchOk) console.log(`          output did not match ${expectMatch}`);
  console.log(r.out.split("\n").map((l) => "          " + l).join("\n"));
}

// --- fixtures -------------------------------------------------------------

const oneSkill = join(SANDBOX, "one");
mkdirSync(join(oneSkill, "nested", "demo"), { recursive: true });
writeFileSync(join(oneSkill, "nested", "demo", "SKILL.md"),
  "---\nname: demo\ndescription: a fixture skill\n---\n\n# demo\nbody\n");

const manySkills = join(SANDBOX, "many");
for (const n of ["alpha", "beta"]) {
  mkdirSync(join(manySkills, n), { recursive: true });
  writeFileSync(join(manySkills, n, "SKILL.md"),
    `---\nname: ${n}\ndescription: fixture\n---\n\n# ${n}\n`);
}

const empty = join(SANDBOX, "empty");
mkdirSync(empty, { recursive: true });

// --- tests ----------------------------------------------------------------

console.log("\nskill-land\n");

t("--list shows the agent table",        ["--list"], 0, /claude-code/);
t("--help exits 0",                      ["--help"], 0, /--for <agent>/);

t("installs a skill found nested",       [oneSkill, "--for", "codex"], 0, /OK/);
t("verify passes on a good install",     [oneSkill, "--for", "codex", "--verify"], 0, /verified/);
t("--dry-run writes nothing",            [oneSkill, "--for", "claude-code", "--dry-run"], 0, /would write/);

t("refuses to guess between skills",     [manySkills, "--for", "codex"], 1, /refusing to guess/);
t("--skill picks one of several",        [manySkills, "--for", "codex", "--skill", "alpha"], 0, /OK/);
t("unknown --skill name fails",          [manySkills, "--for", "codex", "--skill", "nope"], 1, /no skill named/);

t("unknown agent fails",                 [oneSkill, "--for", "nope"], 1, /unknown agent/);
t("missing --for fails",                 [oneSkill], 1, /no --for/);
t("unparseable source fails",            ["not a repo!!", "--for", "codex"], 1, /cannot parse source/);
t("no SKILL.md anywhere fails",          [empty, "--for", "codex"], 1, /no SKILL.md found/);

// Verification must fail when the installed file no longer matches the source.
{
  run([oneSkill, "--for", "codex"]);
  const target = join(HOME, ".codex", "skills", "demo", "SKILL.md");
  if (!existsSync(target)) { fail++; console.log("  FAIL  fixture install did not land"); }
  else {
    writeFileSync(target, readFileSync(target, "utf-8").replace("body", "TAMPERED"));
    t("verify catches a tampered install", [oneSkill, "--for", "codex", "--verify"], 1, /content matches source/);
  }
}

// A skill reachable only through a symlink must still be found.
{
  const linked = join(SANDBOX, "linked");
  mkdirSync(join(linked, "s"), { recursive: true });
  execFileSync("ln", ["-s", join(oneSkill, "nested", "demo", "SKILL.md"), join(linked, "s", "SKILL.md")]);
  t("finds a symlinked SKILL.md", [linked, "--for", "codex", "--dry-run"], 0, /would write/);
}

rmSync(SANDBOX, { recursive: true, force: true });

console.log(`\n  ${pass} passed, ${fail} failed\n`);
process.exit(fail ? 1 : 0);
