use anyhow::Result;
use std::process::{Command, ExitStatus};

pub fn run_git_cmd(args: &[String]) -> Result<ExitStatus> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    let status = cmd.spawn()?.wait()?;
    Ok(status)
}

#[allow(dead_code)]
pub fn git_output(args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
