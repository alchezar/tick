//! Thin wrapper around the `gh` CLI for resolving PR metadata.

use std::process::Command;

/// Resolves the source branch of a pull request via the `gh` CLI.
///
/// Returns `None` when `gh` is not installed, not authenticated, the repo is
/// inaccessible, or the response cannot be parsed - the caller treats the
/// branch as simply unknown rather than surfacing an error.
#[must_use]
pub fn fetch_branch_name(repo_url: &str, pr_number: u32) -> Option<String> {
    let output = Command::new("gh")
        .args([
            "pr",
            "view",
            &pr_number.to_string(),
            "--repo",
            repo_url,
            "--json",
            "headRefName",
            "-q",
            ".headRefName",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!branch.is_empty()).then_some(branch)
}
