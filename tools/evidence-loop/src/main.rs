mod board;
mod cli;
mod evidence;
mod experiment;
mod git;
mod project;
mod state;
mod validation;

use clap::Parser;
use cli::{Cli, Commands};
use experiment::{ArtifactInfo, Classification, ClosureInfo, ClosureStatus, ExperimentRecord, HypothesisStatus, Mechanism, RawInfo, ResultInfo, ReviewKind};
use project::Project;
use state::{check_transition, next_command_for, Command as StepCommand};
use std::str::FromStr;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init => cmd_init(),
        Commands::New { hypothesis } => cmd_new(hypothesis),
        Commands::Status { json } => cmd_status(json),
        Commands::Next { json, experiment } => cmd_next(json, experiment),
        Commands::Verify { experiment, path, baseline } => cmd_verify(experiment, path, baseline),
        Commands::Result {
            experiment,
            classification,
            mechanism,
            hypothesis_status,
            observation,
            interpretation,
            override_flag,
            override_reason,
        } => cmd_result(
            experiment,
            classification,
            mechanism,
            hypothesis_status,
            observation,
            interpretation,
            override_flag,
            override_reason,
        ),
        Commands::ReviewResult { experiment, reviewer, self_reviewed, notes } => {
            cmd_review_result(experiment, reviewer, self_reviewed, notes)
        }
        Commands::CommitArtifact { experiment } => cmd_commit_artifact(experiment),
        Commands::Close {
            experiment,
            status,
            established,
            not_established,
            remaining_questions,
        } => cmd_close(experiment, status, established, not_established, remaining_questions),
        Commands::ReviewClosure { experiment, reviewer, self_reviewed, notes } => {
            cmd_review_closure(experiment, reviewer, self_reviewed, notes)
        }
        Commands::Gate { experiment, json } => cmd_gate(experiment, json),
    }
}

fn cmd_init() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    Project::init(&cwd)?;
    println!("initialized evidence-loop project at {}", cwd.display());
    Ok(())
}

fn cmd_new(hypothesis: String) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let id = project.next_experiment_id()?;
    let record = ExperimentRecord::new(id.clone(), hypothesis);
    project.save_experiment(&record)?;
    project.set_current_experiment(&id)?;
    board::write_board(&project, &record)?;
    println!("{id} created, state {}", record.state);
    Ok(())
}

fn resolve_experiment(project: &Project, explicit: Option<String>) -> anyhow::Result<String> {
    if let Some(id) = explicit {
        return Ok(id);
    }
    project
        .current_experiment_id()
        .ok_or_else(|| anyhow::anyhow!("no current experiment; run `evidence-loop new \"<hypothesis>\"` first"))
}

fn cmd_status(json: bool) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let Some(id) = project.current_experiment_id() else {
        if json {
            println!("{}", serde_json::json!({ "experiment": null }));
        } else {
            println!("no current experiment");
        }
        return Ok(());
    };
    let record = project.load_experiment(&id)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&record)?);
    } else {
        println!("experiment: {}", record.id);
        println!("hypothesis: {}", record.hypothesis);
        println!("state:      {}", record.state);
    }
    Ok(())
}

fn cmd_next(json: bool, explicit: Option<String>) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let id = resolve_experiment(&project, explicit)?;
    let record = project.load_experiment(&id)?;
    let next = next_command_for(record.state);

    if json {
        let value = serde_json::json!({
            "experiment": record.id,
            "state": record.state.as_str(),
            "next_command": next.map(|c| c.name()),
            "allowed": next.is_some(),
            "required_facts": next.map(|c| c.required_facts()).unwrap_or(&[]),
            "forbidden": next.map(|c| c.forbidden()).unwrap_or(&[]),
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("EXPERIMENT\n{}\n", record.id);
    println!("STATE\n{}\n", record.state);
    match next {
        Some(c) => {
            println!("NEXT ACTION\n{} {}\n", c.name(), record.id);
            println!("REQUIRED");
            for f in c.required_facts() {
                println!("- {f}");
            }
            println!("\nDO NOT");
            for f in c.forbidden() {
                println!("- {f}");
            }
        }
        None => println!("NEXT ACTION\nnone — experiment is at HYPOTHESIS_GATE; propose a new hypothesis with `evidence-loop new`"),
    }
    Ok(())
}

fn cmd_verify(experiment: String, path: String, baseline: Option<String>) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let mut record = project.load_experiment(&experiment)?;
    check_transition(&record.id, record.state, StepCommand::Verify)?;

    let full_path = project.root.join(&path);
    if !full_path.exists() {
        anyhow::bail!("artifact not found: {}", full_path.display());
    }
    let bytes = evidence::read_bytes(&full_path)?;
    let hash = evidence::hash_file(&full_path)?;
    let structure = evidence::inspect_structure(&bytes);
    let baseline_commit = baseline.unwrap_or_else(|| git::short_head(&project.root));

    record.raw = Some(RawInfo {
        path: path.clone(),
        baseline_commit,
        hash,
        components: structure.components,
        rows: structure.rows,
        terminus: structure.terminus,
        verified_at: chrono::Utc::now(),
    });
    record.record_transition(StepCommand::Verify);
    project.save_experiment(&record)?;
    board::write_board(&project, &record)?;

    let raw = record.raw.as_ref().unwrap();
    println!("RAW VERIFICATION\n");
    println!("Artifact: {}", raw.path);
    println!("Baseline: {}", raw.baseline_commit);
    println!("Components: {}", raw.components);
    println!("Rows: {}", raw.rows);
    println!("Terminus: {}", raw.terminus);
    println!("\nResult: PASS -> state {}", record.state);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_result(
    experiment: String,
    classification: String,
    mechanism: String,
    hypothesis_status: String,
    observation: String,
    interpretation: String,
    override_flag: bool,
    override_reason: Option<String>,
) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let mut record = project.load_experiment(&experiment)?;
    check_transition(&record.id, record.state, StepCommand::Result)?;

    let classification = Classification::from_str(&classification)?;
    let mechanism = Mechanism::from_str(&mechanism)?;
    let hypothesis_status = HypothesisStatus::from_str(&hypothesis_status)?;

    validation::validate_result(classification, mechanism, hypothesis_status, override_flag, &override_reason)?;

    record.result = Some(ResultInfo {
        classification,
        mechanism,
        hypothesis_status,
        observation,
        interpretation,
        override_applied: override_flag,
        override_reason,
        recorded_at: chrono::Utc::now(),
        reviewed_at: None,
        reviewer: None,
        review_kind: None,
    });
    record.record_transition(StepCommand::Result);
    project.save_experiment(&record)?;
    board::write_board(&project, &record)?;
    println!("{} result recorded -> state {}", record.id, record.state);
    Ok(())
}

fn cmd_review_result(experiment: String, reviewer: String, self_reviewed: bool, notes: Option<String>) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let mut record = project.load_experiment(&experiment)?;
    check_transition(&record.id, record.state, StepCommand::ReviewResult)?;

    let result = record
        .result
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("no result recorded for {}", record.id))?;
    result.reviewed_at = Some(chrono::Utc::now());
    result.reviewer = Some(reviewer);
    result.review_kind = Some(if self_reviewed { ReviewKind::SelfReviewed } else { ReviewKind::Independent });
    if let Some(notes) = notes {
        result.interpretation.push_str(&format!("\n\nReview notes: {notes}"));
    }
    record.record_transition(StepCommand::ReviewResult);
    project.save_experiment(&record)?;
    board::write_board(&project, &record)?;
    println!("{} result reviewed -> state {}", record.id, record.state);
    Ok(())
}

fn cmd_commit_artifact(experiment: String) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let mut record = project.load_experiment(&experiment)?;
    check_transition(&record.id, record.state, StepCommand::CommitArtifact)?;

    let raw = record
        .raw
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no raw artifact recorded for {}", record.id))?;
    let full_path = project.root.join(&raw.path);
    if !full_path.exists() {
        anyhow::bail!("artifact no longer exists at {}", full_path.display());
    }
    let current_hash = evidence::hash_file(&full_path)?;
    if current_hash != raw.hash {
        anyhow::bail!(
            "artifact at {} has changed since verification (was {}, now {}); this is new evidence, not the verified artifact",
            raw.path,
            raw.hash,
            current_hash
        );
    }

    let commit_sha = git::commit_artifact(&project.root, &raw.path, &record.id)?;
    record.artifact = Some(ArtifactInfo {
        committed_at: chrono::Utc::now(),
        commit_sha: commit_sha.clone(),
    });
    record.record_transition(StepCommand::CommitArtifact);
    project.save_experiment(&record)?;
    board::write_board(&project, &record)?;
    println!("{} artifact committed ({commit_sha}) -> state {}", record.id, record.state);
    Ok(())
}

fn cmd_close(
    experiment: String,
    status: String,
    established: String,
    not_established: String,
    remaining_questions: String,
) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let mut record = project.load_experiment(&experiment)?;
    check_transition(&record.id, record.state, StepCommand::Close)?;

    let status = ClosureStatus::from_str(&status)?;
    record.closure = Some(ClosureInfo {
        status,
        established,
        not_established,
        remaining_questions,
        recorded_at: chrono::Utc::now(),
        reviewed_at: None,
        reviewer: None,
        review_kind: None,
    });
    record.record_transition(StepCommand::Close);
    project.save_experiment(&record)?;
    board::write_board(&project, &record)?;
    println!("{} closure recorded -> state {}", record.id, record.state);
    Ok(())
}

fn cmd_review_closure(experiment: String, reviewer: String, self_reviewed: bool, notes: Option<String>) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let mut record = project.load_experiment(&experiment)?;
    check_transition(&record.id, record.state, StepCommand::ReviewClosure)?;

    let closure = record
        .closure
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("no closure recorded for {}", record.id))?;
    closure.reviewed_at = Some(chrono::Utc::now());
    closure.reviewer = Some(reviewer);
    closure.review_kind = Some(if self_reviewed { ReviewKind::SelfReviewed } else { ReviewKind::Independent });
    if let Some(notes) = notes {
        closure.remaining_questions.push_str(&format!("\n\nReview notes: {notes}"));
    }
    record.record_transition(StepCommand::ReviewClosure);
    project.save_experiment(&record)?;
    board::write_board(&project, &record)?;
    println!("{} closure reviewed -> state {}", record.id, record.state);
    Ok(())
}

fn cmd_gate(experiment: String, json: bool) -> anyhow::Result<()> {
    let project = Project::discover(&std::env::current_dir()?)?;
    let mut record = project.load_experiment(&experiment)?;
    check_transition(&record.id, record.state, StepCommand::Gate)?;

    record.record_transition(StepCommand::Gate);
    project.save_experiment(&record)?;
    board::write_board(&project, &record)?;

    let result_review_kind = record.result.as_ref().and_then(|r| r.review_kind);
    let closure_review_kind = record.closure.as_ref().and_then(|c| c.review_kind);

    if json {
        let value = serde_json::json!({
            "experiment": record.id,
            "state": record.state.as_str(),
            "raw_verified": record.raw.is_some(),
            "result_reviewed": record.result.as_ref().and_then(|r| r.reviewed_at).is_some(),
            "result_review_kind": result_review_kind.map(|k| k.as_str()),
            "artifact_committed": record.artifact.is_some(),
            "closure_reviewed": record.closure.as_ref().and_then(|c| c.reviewed_at).is_some(),
            "closure_review_kind": closure_review_kind.map(|k| k.as_str()),
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    let review_label = |kind: Option<ReviewKind>| match kind {
        Some(ReviewKind::SelfReviewed) => " (SELF-REVIEWED, no independent reviewer available)",
        _ => "",
    };

    println!("HYPOTHESIS GATE\n");
    println!("Raw artifact verified: YES");
    println!("Result reviewed:       YES{}", review_label(result_review_kind));
    println!("Artifact committed:    YES");
    println!("Closure reviewed:      YES{}", review_label(closure_review_kind));
    if let Some(r) = &record.result {
        println!("\nClassification: {}", r.classification);
        println!("Mechanism exercised: {}", if matches!(r.mechanism, Mechanism::Exercised) { "YES" } else { "NO" });
        println!("Hypothesis status: {}", r.hypothesis_status);
    }
    println!("\nDecision: a new hypothesis may now be proposed with `evidence-loop new`.");
    Ok(())
}
