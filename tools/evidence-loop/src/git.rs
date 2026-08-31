use std::path::Path;
use std::process::Command;

fn run(dir: &Path, args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git").current_dir(dir).args(args).output()?;
    if !out.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The current HEAD short SHA, or "unknown" if this isn't a git repo yet /
/// has no commits. Verification should never fail just because the project
/// hasn't made its first commit.
pub fn short_head(dir: &Path) -> String {
    run(dir, &["rev-parse", "--short", "HEAD"]).unwrap_or_else(|_| "unknown".into())
}

/// Stage and commit exactly the given raw artifact path, separately from
/// any interpretation. Returns the resulting commit's short SHA.
pub fn commit_artifact(dir: &Path, relative_path: &str, experiment_id: &str) -> anyhow::Result<String> {
    run(dir, &["add", "--", relative_path])?;
    let message = format!("evidence({experiment_id}): commit raw artifact");
    run(dir, &["commit", "-m", &message, "--", relative_path])?;
    run(dir, &["rev-parse", "--short", "HEAD"])
}
