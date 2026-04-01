use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashMap;

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

pub fn get_upstream_remote(branch: &str) -> Result<Option<String>> {
    // git rev-parse --abbrev-ref --symbolic-full-name <branch>@{u}
    let key = format!("{}@{{u}}", branch);
    let res = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", &key])
        .output();
    if let Ok(out) = res {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            return Ok(None);
        }
        // format: remote/branch
        if let Some(pos) = s.find('/') {
            let remote = &s[..pos];
            return Ok(Some(remote.to_string()));
        }
    }
    Ok(None)
}

pub fn list_remotes() -> Result<HashMap<String, String>> {
    let out = std::process::Command::new("git")
        .args(["remote", "-v"])
        .output()?;
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    Ok(parse_remotes(&s))
}

fn url_path_segments(url: &str) -> Vec<String> {
    // take last 3 path segments from URL
    // support ssh and https
    // examples: git@host:owner/repo.git, https://host/owner/repo.git
    let re = Regex::new(r"[:/](?P<path>[^/:]+/[^/:]+(?:/[^/:]+)?)$").unwrap();
    if let Some(cap) = re.captures(url) {
        let path = cap.name("path").unwrap().as_str();
        return path.split('/').map(|s| s.to_string()).collect();
    }
    // fallback: split by / and take last 3
    url.split('/')
        .rev()
        .take(3)
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
}

pub fn are_urls_similar(a: &str, b: &str) -> bool {
    let asg = url_path_segments(a);
    let bsg = url_path_segments(b);
    let mut same = 0;
    for (i, seg) in asg.iter().rev().enumerate() {
        if i >= bsg.len() {
            break;
        }
        if seg == &bsg[bsg.len() - 1 - i] {
            same += 1;
        }
    }
    same >= 1
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
    // 4. env / git config xgit.remote
    if let Ok(r) = std::env::var("XGIT_REMOTE") {
        if !r.is_empty() {
            return Ok(r);
        }
    }
    let cfg = std::process::Command::new("git")
        .args(["config", "--get", "xgit.remote"])
        .output()?;
    let cfgs = String::from_utf8_lossy(&cfg.stdout).trim().to_string();
    if !cfgs.is_empty() {
        return Ok(cfgs);
    }

    // 5. auto: git remote -v
    let remotes = list_remotes()?;
    if remotes.is_empty() {
        return Err(anyhow!("no git remotes found"));
    }
    if remotes.len() == 1 {
        return Ok(remotes.keys().next().unwrap().to_string());
    }

    // If multiple remotes, try to compare to origin URL if present
    if let Some(origin_url) = remotes.get("origin") {
        for (name, url) in &remotes {
            if are_urls_similar(origin_url, url) {
                return Ok(name.clone());
            }
        }
    }

    // fallback prefer list
    let prefer = vec!["origin", "origin2", "upstream"];
    for p in prefer {
        if remotes.contains_key(p) {
            return Ok(p.to_string());
        }
    }

    // last resort: return first
    Ok(remotes.keys().next().unwrap().to_string())
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
    fn test_url_segments() {
        let a = "git@host:owner/repo.git";
        let b = "https://host/owner/repo.git";
        assert!(are_urls_similar(a, b));
    }
}
