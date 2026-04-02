use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn run_git(cwd: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to execute git command")
}

fn run_git_ok(cwd: &Path, args: &[&str]) {
    let output = run_git(cwd, args);
    assert!(
        output.status.success(),
        "git command failed: git {}\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_xgit(cwd: &Path, args: &[&str]) -> Output {
    let exe = env!("CARGO_BIN_EXE_xgit");
    Command::new(exe)
        .current_dir(cwd)
        .env("XGIT_LANG", "en-US")
        .args(args)
        .output()
        .expect("failed to execute xgit command")
}

fn current_branch(cwd: &Path) -> String {
    let output = run_git(cwd, &["branch", "--show-current"]);
    assert!(output.status.success(), "failed to query current branch");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn init_repo_with_origin_upstream() -> (TempDir, TempDir, String) {
    let repo = TempDir::new().expect("create repo tempdir");
    let remote = TempDir::new().expect("create remote tempdir");
    run_git_ok(remote.path(), &["init", "--bare"]);

    run_git_ok(repo.path(), &["init"]);
    run_git_ok(repo.path(), &["config", "user.name", "xgit-test"]);
    run_git_ok(repo.path(), &["config", "user.email", "xgit-test@example.com"]);
    std::fs::write(repo.path().join("README.md"), "seed\n").expect("write seed file");
    run_git_ok(repo.path(), &["add", "README.md"]);
    run_git_ok(repo.path(), &["commit", "-m", "init"]);
    let initial_branch = current_branch(repo.path());

    run_git_ok(repo.path(), &["remote", "add", "origin", remote.path().to_str().unwrap()]);
    run_git_ok(repo.path(), &["push", "-u", "origin", &initial_branch]);

    (repo, remote, initial_branch)
}

fn create_remote_branch_and_fetch(
    repo: &Path,
    remote_path: &Path,
    base_branch: &str,
    remote_branch: &str,
) {
    let contributor = TempDir::new().expect("create contributor tempdir");
    run_git_ok(
        contributor.path(),
        &["clone", remote_path.to_str().unwrap(), "."],
    );
    run_git_ok(
        contributor.path(),
        &["config", "user.name", "xgit-contributor"],
    );
    run_git_ok(
        contributor.path(),
        &["config", "user.email", "xgit-contributor@example.com"],
    );
    run_git_ok(contributor.path(), &["checkout", "-b", remote_branch, base_branch]);
    std::fs::write(
        contributor.path().join(format!("{remote_branch}.txt")),
        "remote branch content\n",
    )
    .expect("write remote branch file");
    run_git_ok(contributor.path(), &["add", "."]);
    run_git_ok(
        contributor.path(),
        &["commit", "-m", &format!("create {remote_branch}")],
    );
    run_git_ok(contributor.path(), &["push", "origin", remote_branch]);

    run_git_ok(repo, &["fetch", "origin", remote_branch]);
}

#[test]
fn reset_succeeds_with_upstream_tracking_branch() {
    let (repo, _remote, _base_branch) = init_repo_with_origin_upstream();
    let output = run_xgit(repo.path(), &["reset"]);
    assert!(
        output.status.success(),
        "xgit reset should succeed when upstream exists.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reset_fails_without_upstream_tracking_branch() {
    let (repo, _remote, _base_branch) = init_repo_with_origin_upstream();
    run_git_ok(repo.path(), &["checkout", "-b", "local-no-upstream"]);

    let output = run_xgit(repo.path(), &["reset"]);
    assert!(!output.status.success(), "xgit reset should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("has no upstream tracking branch"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn checkout_remote_creates_local_branch_from_remote_branch() {
    let (repo, remote, base_branch) = init_repo_with_origin_upstream();
    create_remote_branch_and_fetch(repo.path(), remote.path(), &base_branch, "feature-remote");

    let output = run_xgit(repo.path(), &["checkout-remote", "feature-remote", "feature-local"]);
    assert!(
        output.status.success(),
        "checkout-remote should succeed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let branch = current_branch(repo.path());
    assert_eq!(branch, "feature-local");
}

#[test]
fn checkout_remote_fails_when_local_branch_exists() {
    let (repo, remote, base_branch) = init_repo_with_origin_upstream();
    create_remote_branch_and_fetch(repo.path(), remote.path(), &base_branch, "dup-remote");
    run_git_ok(repo.path(), &["checkout", "-b", "dup-remote"]);
    run_git_ok(repo.path(), &["checkout", &base_branch]);

    let output = run_xgit(repo.path(), &["checkout-remote", "dup-remote"]);
    assert!(!output.status.success(), "checkout-remote should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"), "unexpected stderr: {stderr}");
}

#[test]
fn checkout_remote_fails_when_candidates_are_ambiguous() {
    let repo = TempDir::new().expect("create repo tempdir");
    let remote_alpha = TempDir::new().expect("create remote alpha");
    let remote_beta = TempDir::new().expect("create remote beta");
    run_git_ok(remote_alpha.path(), &["init", "--bare"]);
    run_git_ok(remote_beta.path(), &["init", "--bare"]);

    run_git_ok(repo.path(), &["init"]);
    run_git_ok(repo.path(), &["config", "user.name", "xgit-test"]);
    run_git_ok(repo.path(), &["config", "user.email", "xgit-test@example.com"]);
    std::fs::write(repo.path().join("README.md"), "seed\n").expect("write seed file");
    run_git_ok(repo.path(), &["add", "README.md"]);
    run_git_ok(repo.path(), &["commit", "-m", "init"]);

    run_git_ok(
        repo.path(),
        &["remote", "add", "alpha", remote_alpha.path().to_str().unwrap()],
    );
    run_git_ok(
        repo.path(),
        &["remote", "add", "beta", remote_beta.path().to_str().unwrap()],
    );
    run_git_ok(repo.path(), &["push", "alpha", "HEAD:refs/heads/shared-remote"]);
    run_git_ok(repo.path(), &["push", "beta", "HEAD:refs/heads/shared-remote"]);
    run_git_ok(repo.path(), &["fetch", "alpha", "shared-remote"]);
    run_git_ok(repo.path(), &["fetch", "beta", "shared-remote"]);

    let output = run_xgit(repo.path(), &["checkout-remote", "shared-remote"]);
    assert!(
        !output.status.success(),
        "checkout-remote should fail when candidates are ambiguous"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ambiguous"), "unexpected stderr: {stderr}");
}
