use anyhow::{anyhow, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchUpstreamMapping {
    pub local_branch: String,
    pub remote: String,
    pub remote_branch: String,
    pub full_upstream_ref: String,
}

fn parse_remotes(output: &str) -> HashMap<String, String> {
    // parse `git remote -v` lines
    // format: name\turl (fetch)
    let mut map = HashMap::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts[0].to_string();
            let url = parts[1].to_string();
            // prefer fetch entry; overwrite is fine
            map.insert(name, url);
        }
    }
    map
}

fn first_remote_name(remotes: &HashMap<String, String>) -> Option<String> {
    let mut names = remotes.keys().cloned().collect::<Vec<_>>();
    names.sort();
    names.into_iter().next()
}

fn parse_upstream_remote_branch(output: &str) -> Option<String> {
    let raw = output.trim();
    if raw.is_empty() {
        return None;
    }
    let mut parts = raw.splitn(2, '/');
    let remote = parts.next().unwrap_or_default().trim();
    let branch = parts.next().unwrap_or_default().trim();
    if remote.is_empty() || branch.is_empty() {
        None
    } else {
        Some(raw.to_string())
    }
}

fn parse_branch_upstream_mapping(
    local_branch: &str,
    full_upstream_ref: &str,
) -> Option<BranchUpstreamMapping> {
    let trimmed = full_upstream_ref.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = trimmed.splitn(2, '/');
    let remote = parts.next()?.trim();
    let remote_branch = parts.next()?.trim();
    if remote.is_empty() || remote_branch.is_empty() {
        return None;
    }
    Some(BranchUpstreamMapping {
        local_branch: local_branch.to_string(),
        remote: remote.to_string(),
        remote_branch: remote_branch.to_string(),
        full_upstream_ref: trimmed.to_string(),
    })
}

fn split_remote_tracking_ref(reference: &str) -> Option<(&str, &str)> {
    let trimmed = reference.trim();
    let mut parts = trimmed.splitn(2, '/');
    let remote = parts.next()?.trim();
    let branch = parts.next()?.trim();
    if remote.is_empty() || branch.is_empty() {
        return None;
    }
    if branch == "HEAD" {
        return None;
    }
    Some((remote, branch))
}

fn remote_branch_candidates_from_refs(refs_output: &str, remote_branch: &str) -> Vec<String> {
    let mut candidates: Vec<String> = refs_output
        .lines()
        .filter_map(split_remote_tracking_ref)
        .filter(|(_, branch)| *branch == remote_branch)
        .map(|(remote, branch)| format!("{remote}/{branch}"))
        .collect();
    candidates.sort();
    candidates.dedup();
    candidates
}

fn local_branch_exists_from_refs(refs_output: &str, branch: &str) -> bool {
    refs_output.lines().any(|line| line.trim() == branch)
}

pub fn get_current_branch() -> Result<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        Err(anyhow!("unable to determine current branch"))
    } else {
        Ok(s)
    }
}

pub fn get_checked_out_local_branch() -> Result<Option<String>> {
    let branch = get_current_branch()?;
    if branch == "HEAD" {
        Ok(None)
    } else {
        Ok(Some(branch))
    }
}

pub fn get_branch_push_remote(branch: &str) -> Result<Option<String>> {
    let key = format!("branch.{}.pushRemote", branch);
    let res = std::process::Command::new("git")
        .args(["config", "--get", &key])
        .output()?;
    let s = String::from_utf8_lossy(&res.stdout).trim().to_string();
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

pub fn get_branch_remote(branch: &str) -> Result<Option<String>> {
    let key = format!("branch.{}.remote", branch);
    let res = std::process::Command::new("git")
        .args(["config", "--get", &key])
        .output()?;
    let s = String::from_utf8_lossy(&res.stdout).trim().to_string();
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

pub fn get_upstream_remote_branch(branch: &str) -> Result<Option<String>> {
    // git rev-parse --abbrev-ref --symbolic-full-name <branch>@{u}
    let key = format!("{}@{{u}}", branch);
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", &key])
        .output()?;
    if !out.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(parse_upstream_remote_branch(&stdout))
}

pub fn get_branch_upstream_mapping(branch: &str) -> Result<Option<BranchUpstreamMapping>> {
    Ok(get_upstream_remote_branch(branch)?
        .and_then(|full_upstream_ref| parse_branch_upstream_mapping(branch, &full_upstream_ref)))
}

pub fn get_upstream_remote(branch: &str) -> Result<Option<String>> {
    Ok(get_branch_upstream_mapping(branch)?.map(|mapping| mapping.remote))
}

pub fn list_remotes() -> Result<HashMap<String, String>> {
    let out = std::process::Command::new("git")
        .args(["remote", "-v"])
        .output()?;
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    Ok(parse_remotes(&s))
}

pub fn branch_has_remote(branch: &str) -> Result<bool> {
    // Check if branch has any configured pushRemote, remote, or upstream
    if get_branch_push_remote(branch)?.is_some() {
        return Ok(true);
    }
    if get_branch_remote(branch)?.is_some() {
        return Ok(true);
    }
    if get_upstream_remote(branch)?.is_some() {
        return Ok(true);
    }
    Ok(false)
}

pub fn detect_preferred_remote() -> Result<Option<String>> {
    if let Ok(r) = std::env::var("XGIT_REMOTE") {
        if !r.is_empty() {
            return Ok(Some(r));
        }
    }
    let cfg = std::process::Command::new("git")
        .args(["config", "--get", "xgit.remote"])
        .output()?;
    let cfgs = String::from_utf8_lossy(&cfg.stdout).trim().to_string();
    if !cfgs.is_empty() {
        return Ok(Some(cfgs));
    }

    let remotes = list_remotes()?;
    if remotes.is_empty() {
        return Err(anyhow!("no git remotes found"));
    }
    if remotes.len() == 1 {
        return Ok(first_remote_name(&remotes));
    }

    for candidate in ["origin", "origin2", "upstream"] {
        if remotes.contains_key(candidate) {
            return Ok(Some(candidate.to_string()));
        }
    }

    Ok(None)
}

pub fn detect_remote_for_branch(branch: &str) -> Result<String> {
    // 1. branch.<branch>.pushRemote
    if let Some(r) = get_branch_push_remote(branch)? {
        return Ok(r);
    }
    // 2. branch.<branch>.remote
    if let Some(r) = get_branch_remote(branch)? {
        return Ok(r);
    }
    // 3. upstream
    if let Some(r) = get_upstream_remote(branch)? {
        return Ok(r);
    }

    // 4. repo preference / env / config
    if let Some(r) = detect_preferred_remote()? {
        return Ok(r);
    }

    // 5. auto fallback: first remote name
    let remotes = list_remotes()?;
    if remotes.is_empty() {
        return Err(anyhow!("no git remotes found"));
    }
    first_remote_name(&remotes).ok_or_else(|| anyhow!("no git remotes found"))
}

pub fn local_branch_exists(branch: &str) -> Result<bool> {
    let out = std::process::Command::new("git")
        .args(["for-each-ref", "--format=%(refname:short)", "refs/heads"])
        .output()?;
    let refs = String::from_utf8_lossy(&out.stdout);
    Ok(local_branch_exists_from_refs(&refs, branch))
}

pub fn list_remote_branch_candidates(remote_branch: &str) -> Result<Vec<String>> {
    let out = std::process::Command::new("git")
        .args(["for-each-ref", "--format=%(refname:short)", "refs/remotes"])
        .output()?;
    let refs = String::from_utf8_lossy(&out.stdout);
    Ok(remote_branch_candidates_from_refs(&refs, remote_branch))
}

pub fn remote_tracking_branch_exists(remote: &str, remote_branch: &str) -> Result<bool> {
    let target = format!("{remote}/{remote_branch}");
    let candidates = list_remote_branch_candidates(remote_branch)?;
    Ok(candidates.iter().any(|candidate| candidate == &target))
}

pub fn is_remote_gerrit(remote: &str) -> Result<bool> {
    let remotes = list_remotes()?;
    if let Some(url) = remotes.get(remote) {
        // check port 29418
        if url.contains(":29418") {
            return Ok(true);
        }
        // check keywords
        let lower = url.to_lowercase();
        if lower.contains("gerrit") || lower.contains("review") || lower.contains("googlesource") {
            return Ok(true);
        }
        return Ok(false);
    }
    Err(anyhow!("remote '{}' not found", remote))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_remotes() {
        let sample = "origin\tgit@host:owner/repo.git (fetch)\norigin\tgit@host:owner/repo.git (push)\norigin2\thttps://host/owner/repo.git (fetch)\n";
        let m = parse_remotes(sample);
        assert_eq!(m.get("origin").unwrap(), "git@host:owner/repo.git");
        assert_eq!(m.get("origin2").unwrap(), "https://host/owner/repo.git");
    }

    #[test]
    fn parse_upstream_remote_branch_supports_full_remote_refs() {
        assert_eq!(
            parse_upstream_remote_branch("origin2/feature/test\n"),
            Some("origin2/feature/test".to_string())
        );
        assert_eq!(parse_upstream_remote_branch(""), None);
        assert_eq!(parse_upstream_remote_branch("origin2"), None);
    }

    #[test]
    fn parse_branch_upstream_mapping_extracts_remote_and_branch() {
        let mapping =
            parse_branch_upstream_mapping("feature/local-clean", "origin2/feature/remote-target")
                .unwrap();
        assert_eq!(mapping.local_branch, "feature/local-clean");
        assert_eq!(mapping.remote, "origin2");
        assert_eq!(mapping.remote_branch, "feature/remote-target");
        assert_eq!(mapping.full_upstream_ref, "origin2/feature/remote-target");
    }

    #[test]
    fn parse_branch_upstream_mapping_rejects_invalid_input() {
        assert_eq!(parse_branch_upstream_mapping("main", ""), None);
        assert_eq!(parse_branch_upstream_mapping("main", "origin2"), None);
    }

    #[test]
    fn split_remote_tracking_ref_ignores_remote_head_alias() {
        assert_eq!(
            split_remote_tracking_ref("origin2/feature/test"),
            Some(("origin2", "feature/test"))
        );
        assert_eq!(split_remote_tracking_ref("origin/HEAD"), None);
        assert_eq!(split_remote_tracking_ref(""), None);
    }

    #[test]
    fn remote_branch_candidates_and_local_existence_from_refs() {
        let remote_refs = "origin/HEAD\norigin/main\norigin2/main\norigin2/feature/test\n";
        let candidates = remote_branch_candidates_from_refs(remote_refs, "main");
        assert_eq!(
            candidates,
            vec!["origin/main".to_string(), "origin2/main".to_string()]
        );

        let local_refs = "main\nfeature/test\n";
        assert!(local_branch_exists_from_refs(local_refs, "main"));
        assert!(!local_branch_exists_from_refs(local_refs, "missing"));
    }
}
