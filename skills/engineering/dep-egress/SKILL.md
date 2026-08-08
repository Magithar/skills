---
name: dep-egress
description: Audits a third-party package for data egress — what it sends off the machine, to which endpoints, whether that is disclosed, and whether it can be turned off. Covers install scripts, runtime telemetry, and transitive dependencies. Use before adding a dependency, or when auditing one already installed.
---

# dep-egress — Dependency Egress Audit

**Trigger:** Use this skill when the question is *what does this dependency send, and to whom*:
- "Is it safe to add [package]?" / "does [package] phone home?"
- "What telemetry is in our dependencies?"
- "Audit [package] before I install it"
- "Does anything in node_modules call out to the network?"
- A dependency was just recommended and the user wants it checked before adopting

**Do NOT activate for:**
- Vulnerability scanning (CVEs, injection, hardcoded secrets) — that is a security review, a different job
- Regulatory compliance (GDPR, CCPA, HIPAA) — that asks whether *your* handling is lawful, not what a dependency emits
- Threat modeling your own architecture (STRIDE, LINDDUN)
- Picking between candidate libraries on quality grounds — that is capability discovery
- License compatibility questions

This skill answers one question: **what leaves the machine because this package is installed.**

---

## The rule that governs everything below

**Every finding must cite the file and line that proves it.** A network call you did not read is not a
finding. If you cannot point at the code, you say you could not verify it — you do not infer egress from
a package's reputation, its category, or the presence of an analytics dependency in its tree.

The inverse matters just as much: **"I found no egress" is a statement about your search, not about the
package.** Report coverage honestly (see Phase 6). Obfuscated, minified, or native code is a gap, not a
clean bill of health.

## The other rule: report shapes, never secrets

You will be reading code that handles tokens, keys and environment variables, and you may be pointed at a
real project with real ones. **Quote the code that builds a payload; never a captured value.**

- Name the field and its class — `Authorization: <bearer token>`, `env.STRIPE_SECRET_KEY (value redacted)`
- Never reproduce the value of anything named like a credential — `*_KEY`, `*_TOKEN`, `*_SECRET`,
  `PASSWORD`, `AUTH`, `CREDENTIAL`, `SESSION`, `COOKIE` — or any high-entropy literal that looks like one
- Never dump `process.env`, a `.env` file, or a captured request body wholesale. Summarize what a payload
  *would* contain by reading the code that assembles it
- If you run the package to observe traffic, redact before reporting, and never paste raw captures

A finding is `file:line` plus the shape of what goes out. It never needs the secret itself, and a report
containing one has turned an audit into a leak.

---

## Phase 0 — Resolve the target

Establish three things before reading anything:

1. **Identity** — ecosystem and exact name (`npm:posthog-js`, `pypi:requests`, `cargo:reqwest`, or a local
   vendored path). Version matters: audits are version-specific. If no version is given, use what is
   installed locally; if nothing is installed, use latest and say so.
2. **Source of truth** — prefer the *published artifact* over the git repo. Packages can ship files that
   are not in the repository. If you can only read the repo, record that as a coverage gap.
3. **Scope** — direct package only, or the full transitive tree? Default to direct + transitive
   install-time scripts, since those execute unconditionally. Full transitive runtime analysis is
   expensive; ask before doing it on a tree with more than ~50 packages.

If the user named a package that does not exist or is ambiguous across ecosystems, ask once and stop.

---

## Phase 1 — Read the disclosed behavior first

Read what the package *claims*, before you look at what it does. Doing this first gives you something to
compare against; doing it after biases you toward confirming whatever you found.

Look for, and quote:
- README sections on telemetry, analytics, privacy, or "data collection"
- A `PRIVACY.md`, `privacy-policy`, or linked policy URL
- Documented opt-out: environment variables (`DO_NOT_TRACK`, `*_TELEMETRY_DISABLED`, `*_ANALYTICS=0`),
  config flags, constructor options
- Any first-run notice the package prints

Record for each claim: **what it says it sends, and what it says you can disable.** If the package
discloses nothing, record that explicitly — "no disclosure found" is itself a finding.

---

## Phase 2 — Install-time execution (highest priority)

**Do this before runtime analysis.** Install scripts run when the package is installed — before you ever
import it, before any code review, and often before the package is pinned. An undisclosed network call
here is categorically worse than the same call at runtime.

Check:
- **npm** — `scripts.preinstall`, `install`, `postinstall`, `prepare` in `package.json`; any file they
  invoke; `node-gyp` and binary-download helpers that fetch from a URL
- **PyPI** — `setup.py` executed at build/install; `pyproject.toml` build backend hooks; any
  `subprocess`/`urllib`/`requests` call at module scope in those files
- **Cargo** — `build.rs` and anything it downloads
- **Any ecosystem** — postinstall banners, funding/analytics pings, "install statistics" beacons

For each: the file, the line, the URL contacted, and what is sent with it (a bare version ping is
different from one carrying a machine ID).

---

## Phase 3 — Runtime egress

Now read the shipped code for network calls. Work from the entry point (`main`/`module`/`exports`,
`__init__.py`, `lib.rs`) outward.

Find every construct that can reach the network:
- `fetch`, `XMLHttpRequest`, `navigator.sendBeacon`, `WebSocket`, `EventSource`, `http`/`https`/`net`
- `requests`, `urllib`, `httpx`, `aiohttp`, `socket`
- `reqwest`, `hyper`, `ureq`, `std::net`
- Shelling out — `child_process`, `subprocess`, `Command` — to `curl`, `wget`, or anything else
- Dynamic loading of remote code: `eval` of a fetched string, remote script tags, `import()` of a URL

For each call site, determine and record:

| Field | What to capture |
|---|---|
| Location | `file:line` |
| Endpoint | literal URL, or how it is constructed if dynamic |
| Trigger | import time, first use, every call, on error, on a timer, on user action |
| Payload | which fields go into the body or query string, by name and class, values redacted |
| Conditional | is it gated behind a flag, env var, opt-in, or DNT check |

**Import-time and error-path calls deserve special attention.** Egress that fires on module import
happens whether or not you use the feature. Egress on the error path fires exactly when something has
gone wrong and the payload is most likely to contain real data.

---

## Phase 4 — Classify the payload

For every call found, classify what goes out. Escalating sensitivity:

1. **Anonymous counters** — version, platform, a random install ID with no join key
2. **Environment fingerprint** — OS, arch, runtime version, CI detection, locale, timezone, screen size
3. **Stable identifiers** — machine ID, MAC-derived hash, hostname, username, persisted UUID
4. **Project metadata** — package name, dependency list, repo URL, git remote, branch, file paths
5. **User-attributable data** — email, account ID, IP as an identifier, session tokens
6. **Content** — source code, file contents, prompts, request bodies, stack traces with locals,
   environment variables

**Levels 4-6 are the ones that surprise people.** A stack trace with locals, or a file path containing a
username or a client name, carries far more than the package's docs typically imply. Do not flatten this
into "sends telemetry" — say which level, and cite the code that assembles the payload. Cite the
assembling code, not a captured payload; values stay redacted per the rule above.

Note whether anything is sent to a **third party** rather than the maintainer's own domain. A package
that pipes to a commercial analytics vendor has a different blast radius than one hitting its own API.

---

## Phase 5 — Transitive egress

The case that catches people: the top-level package discloses nothing and does nothing, but pulls in a
dependency that does.

1. Resolve the dependency tree (`npm ls`, `pip show`/`uv tree`, `cargo tree`)
2. Flag any dependency whose *purpose* is data collection — analytics, error reporting, session replay,
   product telemetry, crash reporting, attribution
3. Run Phase 2 (install scripts) across the whole tree — it is cheap and it is where the worst findings are
4. For runtime, follow only the packages actually reachable from the entry point. A dev-only or
   optional-peer dependency that never loads in production is a different risk, and you must say which
   it is rather than reporting them alike

Report any transitive egress **that the top-level package does not disclose** as its own finding. That
gap between "what the thing you chose says" and "what installing it actually does" is the point of the
whole audit.

---

## Phase 6 — Coverage check (do not skip)

Before writing the verdict, state plainly what you could not examine:

- Minified or bundled `dist/` output with no readable source
- Obfuscated code, or strings assembled at runtime to hide an endpoint
- Native binaries, `.node` addons, compiled `.so`/`.dylib`, WASM blobs
- Anything downloaded at install time and therefore not present in the published artifact
- Code paths behind a paid tier, feature flag, or platform you did not read
- Transitive packages you deliberately skipped

**A package whose network layer is inside a native binary must not be reported as "no egress found."**
Report it as "not analyzable by this method" — a different and much weaker statement.

---

## Phase 7 — Verdict

Lead with the verdict line, then the evidence. Format:

```
PACKAGE   <ecosystem>:<name>@<version>
EGRESS    NONE FOUND | DISCLOSED | PARTIALLY DISCLOSED | UNDISCLOSED | NOT ANALYZABLE
OPT-OUT   documented <how> | present but undocumented <how> | none found | n/a
SEVERITY  see below
```

**Severity** is driven by disclosure and data class together, not by either alone:

| | Level 1-2 payload | Level 3-4 payload | Level 5-6 payload |
|---|---|---|---|
| **Disclosed, opt-out exists** | note | low | medium |
| **Disclosed, no opt-out** | low | medium | high |
| **Undisclosed** | medium | high | critical |
| **Install-time, undisclosed** | high | critical | critical |

Then, in this order:

1. **Findings** — one per egress path, each with `file:line`, endpoint, trigger, payload class, and
   whether it is disclosed. Most severe first.
2. **Disclosed vs. actual** — an explicit diff. Things it says it sends and does; things it sends and
   never mentions; things it claims to send that you found no code for.
3. **Turning it off** — the name of the env var, flag, or config that disables it, and whether you
   verified it actually gates the call. An opt-out that the code ignores is a critical finding, not a
   mitigation. Names of switches, never values of secrets.
4. **Coverage** — Phase 6, verbatim. Not a footnote.
5. **Recommendation** — one paragraph. Adopt / adopt with this config / do not adopt / needs a human. Say
   which, and why.

Do not soften a finding because the package is popular, well-regarded, or maintained by a large company.
Report what the code does.

---

## Failure modes to avoid

- **Reputation as evidence.** "It's from a major vendor so the telemetry is fine" is not an audit result.
- **Category inference.** Do not assume a package sends telemetry because similar ones do, or that it
  does not because it looks small. Read it.
- **Counting an analytics dependency as a finding.** Its *presence* in the tree is a lead. The finding is
  the call site that reaches it.
- **Confusing opt-out with off-by-default.** Note which. "Set `X=0` to disable" means it is on right now.
- **Silent scope reduction.** If you audited the direct package only, say so in the verdict; do not let a
  reader assume tree coverage.
- **Treating unreadable as clean.** Covered in Phase 6, and it is the most common way this kind of audit
  gets quietly wrong.

---

## Companion skills

Run [SKILLmama](https://github.com/Magithar/SKILLmama) first to pick a library on compatibility,
popularity, maintenance and simplicity. Run this on the winner. The two are deliberately separate: quality
scoring says nothing about what a package emits, and egress says nothing about whether it is any good.

---

*Evaluation method adapted from the approach used by [Privacy Guides](https://www.privacyguides.org)
(CC BY-SA 4.0) — compare a tool's disclosed behavior against its actual behavior, and treat undisclosed
data flows as the finding. Restated here for software dependencies; no text reproduced. Not affiliated
with or endorsed by Privacy Guides.*
