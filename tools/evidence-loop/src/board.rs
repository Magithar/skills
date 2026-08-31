use crate::experiment::{ExperimentRecord, ReviewKind};
use crate::project::Project;
use crate::state::{next_command_for, Command};
use std::fs;

const OWNED_HEADINGS: &[&str] = &[
    "Current Experiment",
    "Hypothesis",
    "Next Action",
    "Evidence",
    "Result",
    "Closure",
    "History",
];

pub fn render_empty() -> String {
    "# Evidence Board\n\n<!-- EVIDENCE-LOOP:STATE\nversion: 1\nexperiment: none\nstate: none\n-->\n\n## Current Experiment\n\n_none — run `evidence-loop new \"<hypothesis>\"`._\n"
        .to_string()
}

fn next_action_text(record: &ExperimentRecord) -> String {
    match next_command_for(record.state) {
        Some(Command::Verify) => "Verify raw artifact.".into(),
        Some(Command::Result) => "Record result.".into(),
        Some(Command::ReviewResult) => "Review result.".into(),
        Some(Command::CommitArtifact) => "Commit raw artifact.".into(),
        Some(Command::Close) => "Record closure.".into(),
        Some(Command::ReviewClosure) => "Review closure.".into(),
        Some(Command::Gate) => "Run hypothesis gate.".into(),
        None => "Experiment complete. Propose a new hypothesis with `evidence-loop new`.".into(),
    }
}

fn reviewed_by_line(reviewer: &Option<String>, review_kind: &Option<ReviewKind>) -> Option<String> {
    let reviewer = reviewer.as_ref()?;
    let kind = match review_kind {
        Some(ReviewKind::SelfReviewed) => "SELF-REVIEWED -- no independent reviewer available",
        _ => "INDEPENDENT",
    };
    Some(format!("{kind} -- {reviewer}"))
}

fn render_owned(record: &ExperimentRecord) -> String {
    let mut out = String::new();

    out.push_str("## Current Experiment\n\n");
    out.push_str(&record.id);
    out.push_str("\n\n");

    out.push_str("## Hypothesis\n\n");
    out.push_str(&record.hypothesis);
    out.push_str("\n\n");

    out.push_str("## Next Action\n\n");
    out.push_str(&next_action_text(record));
    out.push_str("\n\n");

    out.push_str("## Evidence\n\n");
    match &record.raw {
        Some(raw) => {
            out.push_str(&format!("- Artifact: `{}`\n", raw.path));
            out.push_str(&format!("- Baseline: `{}`\n", raw.baseline_commit));
            out.push_str(&format!("- Hash: `{}`\n", raw.hash));
            out.push_str(&format!("- Components: {}\n", raw.components));
            out.push_str(&format!("- Rows: {}\n", raw.rows));
            out.push_str(&format!("- Terminus: {}\n", raw.terminus));
        }
        None => out.push_str("Pending\n"),
    }
    out.push('\n');

    out.push_str("## Result\n\n");
    match &record.result {
        Some(r) => {
            out.push_str("### Classification\n\n");
            out.push_str(&format!("{}\n\n", r.classification));
            out.push_str("### Mechanism Exercised\n\n");
            out.push_str(&format!(
                "{}\n\n",
                if matches!(r.mechanism, crate::experiment::Mechanism::Exercised) { "YES" } else { "NO" }
            ));
            out.push_str("### Hypothesis Status\n\n");
            out.push_str(&format!("{}\n\n", r.hypothesis_status));
            out.push_str("### Observation\n\n");
            out.push_str(&r.observation);
            out.push_str("\n\n");
            out.push_str("### Interpretation\n\n");
            out.push_str(&r.interpretation);
            out.push('\n');
            if let Some(line) = reviewed_by_line(&r.reviewer, &r.review_kind) {
                out.push_str("\n### Reviewed By\n\n");
                out.push_str(&line);
                out.push('\n');
            }
            if let Some(notes) = &r.review_notes {
                out.push_str("\n### Review Notes\n\n");
                out.push_str(notes);
                out.push('\n');
            }
        }
        None => out.push_str("Pending\n"),
    }
    out.push('\n');

    out.push_str("## Closure\n\n");
    match &record.closure {
        Some(c) => {
            out.push_str(&format!("### Status\n\n{}\n\n", c.status));
            out.push_str(&format!("### Established\n\n{}\n\n", c.established));
            out.push_str(&format!("### Not Established\n\n{}\n\n", c.not_established));
            out.push_str(&format!("### Remaining Questions\n\n{}\n", c.remaining_questions));
            if let Some(line) = reviewed_by_line(&c.reviewer, &c.review_kind) {
                out.push_str("\n### Reviewed By\n\n");
                out.push_str(&line);
                out.push('\n');
            }
            if let Some(notes) = &c.review_notes {
                out.push_str("\n### Review Notes\n\n");
                out.push_str(notes);
                out.push('\n');
            }
        }
        None => out.push_str("Pending\n"),
    }
    out.push('\n');

    out.push_str("## History\n\n");
    if record.history.is_empty() {
        out.push_str("_none_\n");
    } else {
        for t in &record.history {
            out.push_str(&format!("- {} -> {} (`{}`)\n", t.from, t.to, t.command));
        }
    }

    out
}

/// Anything the human wrote under a heading `evidence-loop` doesn't own is
/// preserved verbatim, in its original order, appended after the owned
/// sections. The state comment, title, and owned headings are always
/// regenerated from the experiment record.
fn extract_unowned_sections(existing: &str) -> String {
    let mut out = String::new();
    let mut current: Option<(String, String)> = None;

    let flush = |current: &mut Option<(String, String)>, out: &mut String| {
        if let Some((heading, body)) = current.take() {
            if !OWNED_HEADINGS.contains(&heading.as_str()) {
                out.push_str(&format!("## {heading}\n{body}"));
            }
        }
    };

    for line in existing.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            flush(&mut current, &mut out);
            current = Some((heading.trim().to_string(), String::new()));
        } else if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    flush(&mut current, &mut out);
    out
}

pub fn write_board(project: &Project, record: &ExperimentRecord) -> anyhow::Result<()> {
    let existing = fs::read_to_string(project.board_path()).unwrap_or_default();
    let unowned = extract_unowned_sections(&existing);

    let mut out = String::new();
    out.push_str("# Evidence Board\n\n");
    out.push_str(&format!(
        "<!-- EVIDENCE-LOOP:STATE\nversion: 1\nexperiment: {}\nstate: {}\n-->\n\n",
        record.id, record.state
    ));
    out.push_str(&render_owned(record));
    if !unowned.is_empty() {
        out.push('\n');
        out.push_str(&unowned);
    }

    fs::write(project.board_path(), out)?;
    Ok(())
}
