use std::process::Command;

#[test]
fn cli_version_matches_cargo_package_version() {
    let exe = env!("CARGO_BIN_EXE_xgit");
    let output = Command::new(exe)
        .arg("--version")
        .output()
        .expect("failed to run xgit --version");

    assert!(output.status.success(), "xgit --version should exit with 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "version output should contain Cargo package.version, got: {stdout}"
    );
}
