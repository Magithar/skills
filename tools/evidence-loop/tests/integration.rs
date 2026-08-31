use std::fs;
use std::path::Path;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_evidence-loop")
}

fn run(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(bin()).current_dir(dir).args(args).output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn setup_project() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    Command::new("git").current_dir(dir.path()).args(["init", "-q"]).status().unwrap();
    Command::new("git").current_dir(dir.path()).args(["config", "user.email", "test@example.com"]).status().unwrap();
    Command::new("git").current_dir(dir.path()).args(["config", "user.name", "Test"]).status().unwrap();
    fs::write(dir.path().join("README.md"), "seed\n").unwrap();
    Command::new("git").current_dir(dir.path()).args(["add", "."]).status().unwrap();
    Command::new("git").current_dir(dir.path()).args(["commit", "-q", "-m", "seed"]).status().unwrap();
    dir
}

/// Walks the full H1 evidence lifecycle end to end: a 1-component, 286-row
/// degenerate result that must stay DEGENERATE/UNEXERCISED/UNTESTED all the
/// way to the hypothesis gate, exactly as specced.
#[test]
fn full_h1_lifecycle() {
    let dir = setup_project();
    let root = dir.path();

    let (ok, out, err) = run(root, &["init"]);
    assert!(ok, "init failed: {err}");
    assert!(root.join("BOARD.md").exists());
    assert!(out.contains("initialized"));

    let (ok, out, _) = run(root, &["new", "H1 -- decomposition improves retrieval"]);
    assert!(ok);
    assert!(out.contains("E001"));

    // Illegal transition: cannot record a result before raw is verified.
    let (ok, _, err) = run(
        root,
        &[
            "result", "E001",
            "--classification", "DEGENERATE",
            "--mechanism", "UNEXERCISED",
            "--hypothesis-status", "UNTESTED",
            "--observation", "n/a",
            "--interpretation", "n/a",
        ],
    );
    assert!(!ok);
    assert!(err.contains("illegal transition"), "expected illegal transition, got: {err}");

    // Produce the raw artifact: 1 component, 286 rows.
    let evidence_dir = root.join("evidence/E001/raw");
    fs::create_dir_all(&evidence_dir).unwrap();
    let rows: Vec<serde_json::Value> = (0..286).map(|i| serde_json::json!({ "row": i })).collect();
    fs::write(evidence_dir.join("result.json"), serde_json::to_vec(&rows).unwrap()).unwrap();

    let (ok, out, err) = run(root, &["verify", "E001", "evidence/E001/raw/result.json"]);
    assert!(ok, "verify failed: {err}");
    assert!(out.contains("Components: 1"));
    assert!(out.contains("Rows: 286"));
    assert!(out.contains("RAW_VERIFIED"));

    // Reject the false-refutation combination outright.
    let (ok, _, err) = run(
        root,
        &[
            "result", "E001",
            "--classification", "DEGENERATE",
            "--mechanism", "UNEXERCISED",
            "--hypothesis-status", "REFUTED",
            "--observation", "n/a",
            "--interpretation", "n/a",
        ],
    );
    assert!(!ok);
    assert!(err.contains("REFUTED"), "expected mechanism/refuted conflict, got: {err}");

    // The correct, honest classification for a degenerate/unexercised run.
    let (ok, out, err) = run(
        root,
        &[
            "result", "E001",
            "--classification", "DEGENERATE",
            "--mechanism", "UNEXERCISED",
            "--hypothesis-status", "UNTESTED",
            "--observation", "1 component x 286 rows, no decomposition branch taken",
            "--interpretation", "mechanism never exercised; no conclusion about H1 is possible",
        ],
    );
    assert!(ok, "result failed: {err}");
    assert!(out.contains("RESULT_RECORDED"));

    let (ok, out, err) = run(root, &["review-result", "E001", "--reviewer", "reviewer@example.com"]);
    assert!(ok, "review-result failed: {err}");
    assert!(out.contains("RESULT_REVIEWED"));

    let (ok, out, err) = run(root, &["commit-artifact", "E001"]);
    assert!(ok, "commit-artifact failed: {err}");
    assert!(out.contains("ARTIFACT_COMMITTED"));

    // The artifact really is committed to git, separately from the sidecar/board.
    let (ok, log, _) = run(root, &["--version"]); // sanity: binary still runs
    assert!(ok);
    let git_log = Command::new("git")
        .current_dir(root)
        .args(["log", "--oneline"])
        .output()
        .unwrap();
    let git_log = String::from_utf8_lossy(&git_log.stdout);
    assert!(git_log.contains("evidence(E001): commit raw artifact"), "git log: {git_log}, --version output: {log}");

    let (ok, out, err) = run(
        root,
        &[
            "close", "E001",
            "--status", "INCONCLUSIVE",
            "--established", "the pipeline runs end to end",
            "--not-established", "whether decomposition improves retrieval",
            "--remaining-questions", "needs a run where the decomposition branch actually fires",
        ],
    );
    assert!(ok, "close failed: {err}");
    assert!(out.contains("CLOSURE_RECORDED"));

    let (ok, out, err) = run(root, &["review-closure", "E001", "--reviewer", "reviewer@example.com"]);
    assert!(ok, "review-closure failed: {err}");
    assert!(out.contains("CLOSURE_REVIEWED"));

    let (ok, out, err) = run(root, &["gate", "E001"]);
    assert!(ok, "gate failed: {err}");
    assert!(out.contains("HYPOTHESIS_GATE") || out.contains("Decision"));

    // status --json reflects the final, honest state.
    let (ok, out, err) = run(root, &["status", "--json"]);
    assert!(ok, "status failed: {err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["state"], "HYPOTHESIS_GATE");
    assert_eq!(value["result"]["classification"], "DEGENERATE");
    assert_eq!(value["result"]["mechanism"], "UNEXERCISED");
    assert_eq!(value["result"]["hypothesis_status"], "UNTESTED");

    // BOARD.md reflects the same, human-readably.
    let board = fs::read_to_string(root.join("BOARD.md")).unwrap();
    assert!(board.contains("HYPOTHESIS_GATE"));
    assert!(board.contains("DEGENERATE"));
    assert!(board.contains("UNTESTED"));
}

#[test]
fn cannot_skip_states() {
    let dir = setup_project();
    let root = dir.path();
    run(root, &["init"]);
    run(root, &["new", "H2"]);

    let evidence_dir = root.join("evidence/E001/raw");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(evidence_dir.join("data.json"), b"[]").unwrap();
    let (ok, _, _) = run(root, &["verify", "E001", "evidence/E001/raw/data.json"]);
    assert!(ok);

    // Cannot jump straight from RAW_VERIFIED to close.
    let (ok, _, err) = run(
        root,
        &[
            "close", "E001",
            "--status", "INCONCLUSIVE",
            "--established", "x",
            "--not-established", "y",
            "--remaining-questions", "z",
        ],
    );
    assert!(!ok);
    assert!(err.contains("illegal transition"));
}

#[test]
fn artifact_hash_immutability_blocks_commit_after_edit() {
    let dir = setup_project();
    let root = dir.path();
    run(root, &["init"]);
    run(root, &["new", "H3"]);

    let evidence_dir = root.join("evidence/E001/raw");
    fs::create_dir_all(&evidence_dir).unwrap();
    let path = evidence_dir.join("data.json");
    fs::write(&path, b"[1,2,3]").unwrap();
    run(root, &["verify", "E001", "evidence/E001/raw/data.json"]);
    run(
        root,
        &[
            "result", "E001",
            "--classification", "CONCLUSIVE",
            "--mechanism", "EXERCISED",
            "--hypothesis-status", "SUPPORTED",
            "--observation", "ok",
            "--interpretation", "ok",
        ],
    );
    run(root, &["review-result", "E001", "--reviewer", "r"]);

    // Someone edits the artifact after review but before commit.
    fs::write(&path, b"[1,2,3,4]").unwrap();

    let (ok, _, err) = run(root, &["commit-artifact", "E001"]);
    assert!(!ok);
    assert!(err.contains("changed since verification"), "expected hash mismatch error, got: {err}");
}

/// A self-review must be distinguishable from an independent one, on the
/// board and in the sidecar, but must not block the transition.
#[test]
fn self_review_is_recorded_distinctly_from_independent_review() {
    let dir = setup_project();
    let root = dir.path();
    run(root, &["init"]);
    run(root, &["new", "H4"]);

    let evidence_dir = root.join("evidence/E001/raw");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(evidence_dir.join("data.json"), b"[1,2,3]").unwrap();
    run(root, &["verify", "E001", "evidence/E001/raw/data.json"]);
    run(
        root,
        &[
            "result", "E001",
            "--classification", "CONCLUSIVE",
            "--mechanism", "EXERCISED",
            "--hypothesis-status", "SUPPORTED",
            "--observation", "ok",
            "--interpretation", "ok",
        ],
    );

    let (ok, out, err) = run(root, &["review-result", "E001", "--reviewer", "me@example.com", "--self"]);
    assert!(ok, "review-result --self failed: {err}");
    assert!(out.contains("RESULT_REVIEWED"));

    let (ok, out, err) = run(root, &["status", "--json"]);
    assert!(ok, "status failed: {err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["result"]["review_kind"], "SELF");

    let board = fs::read_to_string(root.join("BOARD.md")).unwrap();
    assert!(board.contains("SELF-REVIEWED"), "board should surface self-review distinctly:\n{board}");

    // Without --self, the same command records an independent review.
    run(root, &["commit-artifact", "E001"]);
    run(
        root,
        &[
            "close", "E001",
            "--status", "CONFIRMED",
            "--established", "x",
            "--not-established", "y",
            "--remaining-questions", "z",
        ],
    );
    let (ok, out, err) = run(root, &["review-closure", "E001", "--reviewer", "someone-else@example.com"]);
    assert!(ok, "review-closure failed: {err}");
    assert!(out.contains("CLOSURE_REVIEWED"));

    let (ok, out, err) = run(root, &["status", "--json"]);
    assert!(ok, "status failed: {err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["closure"]["review_kind"], "INDEPENDENT");

    let (ok, out, err) = run(root, &["gate", "E001", "--json"]);
    assert!(ok, "gate failed: {err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["result_review_kind"], "SELF");
    assert_eq!(value["closure_review_kind"], "INDEPENDENT");
}

/// --notes must land in its own field/board section, not get appended into
/// interpretation or remaining_questions prose.
#[test]
fn review_notes_get_their_own_section_not_appended_to_prose() {
    let dir = setup_project();
    let root = dir.path();
    run(root, &["init"]);
    run(root, &["new", "H5"]);

    let evidence_dir = root.join("evidence/E001/raw");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(evidence_dir.join("data.json"), b"[1,2,3]").unwrap();
    run(root, &["verify", "E001", "evidence/E001/raw/data.json"]);
    run(
        root,
        &[
            "result", "E001",
            "--classification", "CONCLUSIVE",
            "--mechanism", "EXERCISED",
            "--hypothesis-status", "SUPPORTED",
            "--observation", "obs-marker",
            "--interpretation", "interp-marker",
        ],
    );
    run(root, &["review-result", "E001", "--reviewer", "r", "--notes", "NOTES-MARKER"]);

    let (ok, out, err) = run(root, &["status", "--json"]);
    assert!(ok, "status failed: {err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["result"]["review_notes"], "NOTES-MARKER");
    assert_eq!(value["result"]["interpretation"], "interp-marker", "notes must not be appended to interpretation");

    let board = fs::read_to_string(root.join("BOARD.md")).unwrap();
    assert!(board.contains("### Review Notes"), "board should have a dedicated notes section:\n{board}");
    assert!(board.contains("NOTES-MARKER"));
    // The interpretation section itself must be exactly the marker, with no
    // "Review notes:" text appended after it.
    let interp_start = board.find("### Interpretation\n\n").unwrap() + "### Interpretation\n\n".len();
    let interp_section = &board[interp_start..];
    let interp_end = interp_section.find("\n\n").unwrap_or(interp_section.len());
    assert_eq!(&interp_section[..interp_end], "interp-marker");
}

/// verify/commit-artifact accept a directory of related raw files as one
/// evidence bundle, hashing the whole set for immutability.
#[test]
fn verify_accepts_a_directory_as_an_evidence_bundle() {
    let dir = setup_project();
    let root = dir.path();
    run(root, &["init"]);
    run(root, &["new", "H6"]);

    let bundle = root.join("evidence/E001/raw");
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("a.txt"), b"aaa").unwrap();
    fs::write(bundle.join("b.txt"), b"bbb").unwrap();

    let (ok, out, err) = run(root, &["verify", "E001", "evidence/E001/raw"]);
    assert!(ok, "verify on a directory failed: {err}");
    assert!(out.contains("Files: 2"), "expected file count in bundle output:\n{out}");
    assert!(out.contains("bundle"), "expected terminus=bundle:\n{out}");

    run(
        root,
        &[
            "result", "E001",
            "--classification", "CONCLUSIVE",
            "--mechanism", "EXERCISED",
            "--hypothesis-status", "SUPPORTED",
            "--observation", "ok",
            "--interpretation", "ok",
        ],
    );
    run(root, &["review-result", "E001", "--reviewer", "r"]);

    let (ok, out, err) = run(root, &["commit-artifact", "E001"]);
    assert!(ok, "commit-artifact on a bundle failed: {err}");
    assert!(out.contains("ARTIFACT_COMMITTED"));

    let git_log = Command::new("git").current_dir(root).args(["show", "--stat", "HEAD"]).output().unwrap();
    let git_log = String::from_utf8_lossy(&git_log.stdout);
    assert!(git_log.contains("a.txt") && git_log.contains("b.txt"), "expected both bundle files in the commit:\n{git_log}");

    // Changing any one file in the bundle must be caught, same as a single artifact.
    let dir2 = setup_project();
    let root2 = dir2.path();
    run(root2, &["init"]);
    run(root2, &["new", "H7"]);
    let bundle2 = root2.join("evidence/E001/raw");
    fs::create_dir_all(&bundle2).unwrap();
    fs::write(bundle2.join("a.txt"), b"aaa").unwrap();
    run(root2, &["verify", "E001", "evidence/E001/raw"]);
    run(
        root2,
        &[
            "result", "E001",
            "--classification", "CONCLUSIVE",
            "--mechanism", "EXERCISED",
            "--hypothesis-status", "SUPPORTED",
            "--observation", "ok",
            "--interpretation", "ok",
        ],
    );
    run(root2, &["review-result", "E001", "--reviewer", "r"]);
    fs::write(bundle2.join("a.txt"), b"tampered").unwrap();
    let (ok, _, err) = run(root2, &["commit-artifact", "E001"]);
    assert!(!ok, "commit-artifact should reject a tampered bundle file");
    assert!(err.contains("changed since verification"));
}

/// --local pins the artifact's hash without writing it into the subject
/// repo's git history.
#[test]
fn commit_artifact_local_does_not_touch_git_history() {
    let dir = setup_project();
    let root = dir.path();
    run(root, &["init"]);
    run(root, &["new", "H8"]);

    let evidence_dir = root.join("evidence/E001/raw");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(evidence_dir.join("data.json"), b"[1,2,3]").unwrap();
    run(root, &["verify", "E001", "evidence/E001/raw/data.json"]);
    run(
        root,
        &[
            "result", "E001",
            "--classification", "CONCLUSIVE",
            "--mechanism", "EXERCISED",
            "--hypothesis-status", "SUPPORTED",
            "--observation", "ok",
            "--interpretation", "ok",
        ],
    );
    run(root, &["review-result", "E001", "--reviewer", "r"]);

    let before = Command::new("git").current_dir(root).args(["rev-parse", "HEAD"]).output().unwrap();
    let before = String::from_utf8_lossy(&before.stdout).trim().to_string();

    let (ok, out, err) = run(root, &["commit-artifact", "E001", "--local"]);
    assert!(ok, "commit-artifact --local failed: {err}");
    assert!(out.contains("ARTIFACT_COMMITTED"));
    assert!(out.contains("not committed to git"), "expected explicit local-only message:\n{out}");

    let after = Command::new("git").current_dir(root).args(["rev-parse", "HEAD"]).output().unwrap();
    let after = String::from_utf8_lossy(&after.stdout).trim().to_string();
    assert_eq!(before, after, "HEAD must not move on --local commit-artifact");

    let (ok, out, err) = run(root, &["status", "--json"]);
    assert!(ok, "status failed: {err}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(value["artifact"]["commit_sha"].is_null(), "commit_sha should be absent/null for a local artifact");

    // The rest of the lifecycle proceeds normally from here.
    let (ok, _, err) = run(
        root,
        &[
            "close", "E001",
            "--status", "CONFIRMED",
            "--established", "x",
            "--not-established", "y",
            "--remaining-questions", "z",
        ],
    );
    assert!(ok, "close after --local commit failed: {err}");
}
