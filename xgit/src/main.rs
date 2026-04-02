mod annotate;
mod code_file_types;
mod config;
mod gitutils;
mod i18n;
mod remote;
mod setup_ui;
mod version;

use anyhow::{anyhow, bail, Result};
use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgMatches, Command};
use clap_complete::{
    generate,
    shells::{Bash, Fish, PowerShell, Zsh},
};
use config::{load_runtime_config, LoadConfigOptions, RuntimeConfig};
use gitutils::run_git_cmd;
use i18n::Catalog;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const PROFILE_BLOCK_BEGIN: &str = "# >>> xgit completion (managed) >>>";
const PROFILE_BLOCK_END: &str = "# <<< xgit completion (managed) <<<";

#[derive(Debug, Clone)]
struct CompletionInstallPlan {
    shell: String,
    script_target_path: PathBuf,
    profile_path: Option<PathBuf>,
    profile_lines: Vec<String>,
}

fn main() -> Result<()> {
    env_logger::init();

    let raw_args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let cwd = std::env::current_dir()?;
    let runtime = load_runtime_config(&cwd, &LoadConfigOptions)?;
    let catalog = i18n::load_catalog(&runtime.effective.ui.lang, &cwd)?;

    let mut command = build_runtime_command(&catalog, &runtime);
    let matches = match command.clone().try_get_matches_from(raw_args) {
        Ok(matches) => matches,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                err.print()?;
                return Ok(());
            }
            _ => return Err(err.into()),
        },
    };

    match matches.subcommand() {
        Some(("push", sub)) => execute_push(sub, &catalog, &runtime)?,
        Some(("setup", sub)) => execute_setup(sub, &catalog, &runtime)?,
        Some(("annotate", sub)) => execute_annotate(sub, &catalog, &runtime, &cwd)?,
        Some(("reset", sub)) => execute_reset(sub, &catalog)?,
        Some(("checkout-remote", sub)) => execute_checkout_remote(sub, &catalog)?,
        Some(("completion", sub)) => execute_completion(sub, &catalog, &runtime)?,
        _ => {
            command.print_help()?;
            println!();
        }
    }

    Ok(())
}

fn build_runtime_command(catalog: &Catalog, runtime: &RuntimeConfig) -> Command {
    let disabled = catalog.t("status.disabled.short");
    let push_about = if runtime.effective.features.push {
        catalog.t("cmd.push.about")
    } else {
        format!("{} {}", catalog.t("cmd.push.about"), disabled)
    };
    let annotate_about = if runtime.effective.features.annotate {
        catalog.t("cmd.annotate.about")
    } else {
        format!("{} {}", catalog.t("cmd.annotate.about"), disabled)
    };

    Command::new("xgit")
        .version(version::app_version())
        .about(catalog.t("app.about"))
        .subcommand(
            Command::new("push")
                .about(push_about)
                .arg(
                    Arg::new("branch")
                        .num_args(1)
                        .value_name("BRANCH")
                        .help(catalog.t("cmd.push.arg.branch.help")),
                )
                .arg(
                    Arg::new("remote")
                        .long("remote")
                        .num_args(1)
                        .value_name("REMOTE")
                        .help(catalog.t("cmd.push.arg.remote.help")),
                )
                .arg(
                    Arg::new("gerrit")
                        .long("gerrit")
                        .action(ArgAction::SetTrue)
                        .help(catalog.t("cmd.push.arg.gerrit.help")),
                )
                .arg(
                    Arg::new("no-thin")
                        .long("no-thin")
                        .action(ArgAction::SetTrue)
                        .help(catalog.t("cmd.push.arg.no_thin.help")),
                )
                .arg(
                    Arg::new("force-with-lease")
                        .long("force-with-lease")
                        .action(ArgAction::SetTrue)
                        .help(catalog.t("cmd.push.arg.force_with_lease.help")),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(ArgAction::SetTrue)
                        .help(catalog.t("cmd.push.arg.dry_run.help")),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(ArgAction::SetTrue)
                        .help(catalog.t("cmd.push.arg.verbose.help")),
                ),
        )
        .subcommand(
            Command::new("setup")
                .about(catalog.t("cmd.setup.about"))
                .arg(
                    Arg::new("project")
                        .long("project")
                        .action(ArgAction::SetTrue)
                        .help(catalog.t("cmd.setup.arg.project.help")),
                ),
        )
        .subcommand(
            Command::new("annotate")
                .about(annotate_about)
                .arg(
                    Arg::new("staged")
                        .long("staged")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("latest-commit")
                        .help(catalog.t("cmd.annotate.arg.staged.help")),
                )
                .arg(
                    Arg::new("latest-commit")
                        .long("latest-commit")
                        .action(ArgAction::SetTrue)
                        .help(catalog.t("cmd.annotate.arg.latest_commit.help")),
                )
                .arg(
                    Arg::new("include-untracked")
                        .long("include-untracked")
                        .action(ArgAction::SetTrue)
                        .help(catalog.t("cmd.annotate.arg.include_untracked.help")),
                )
                .arg(
                    Arg::new("reason")
                        .long("reason")
                        .num_args(1)
                        .value_name("TEXT")
                        .help(catalog.t("cmd.annotate.arg.reason.help")),
                )
                .arg(
                    Arg::new("reference-kind")
                        .long("reference-kind")
                        .num_args(1)
                        .value_name("KIND")
                        .help(catalog.t("cmd.annotate.arg.reference_kind.help")),
                )
                .arg(
                    Arg::new("reference-value")
                        .long("reference-value")
                        .num_args(1)
                        .value_name("VALUE")
                        .help(catalog.t("cmd.annotate.arg.reference_value.help")),
                ),
        )
        .subcommand(
            Command::new("reset").about(catalog.t("cmd.reset.about")).arg(
                Arg::new("hard")
                    .long("hard")
                    .action(ArgAction::SetTrue)
                    .help(catalog.t("cmd.reset.arg.hard.help")),
            ),
        )
        .subcommand(
            Command::new("checkout-remote")
                .about(catalog.t("cmd.checkout_remote.about"))
                .arg(
                    Arg::new("remote-branch")
                        .num_args(1)
                        .required(true)
                        .value_name("REMOTE_BRANCH")
                        .help(catalog.t("cmd.checkout_remote.arg.remote_branch.help")),
                )
                .arg(
                    Arg::new("local-branch")
                        .num_args(1)
                        .value_name("LOCAL_BRANCH")
                        .help(catalog.t("cmd.checkout_remote.arg.local_branch.help")),
                ),
        )
        .subcommand(
            Command::new("completion")
                .about(catalog.t("cmd.completion.about"))
                .arg(
                    Arg::new("install")
                        .long("install")
                        .action(ArgAction::SetTrue)
                        .help(catalog.t("cmd.completion.arg.install.help")),
                )
                .arg(
                    Arg::new("shell")
                        .num_args(1)
                        .required_unless_present("install")
                        .value_name("SHELL")
                        .help(catalog.t("cmd.completion.arg.shell.help")),
                ),
        )
}

fn execute_push(sub: &ArgMatches, catalog: &Catalog, runtime: &RuntimeConfig) -> Result<()> {
    if !runtime.effective.features.push {
        bail!(
            "{}",
            catalog.tf(
                "error.feature.disabled",
                &[("feature", catalog.t("feature.push"))]
            )
        );
    }

    if which::which("git").is_err() {
        bail!("{}", catalog.t("error.git.not_found"));
    }

    let branch = match sub.get_one::<String>("branch") {
        Some(value) => value.clone(),
        None => {
            let current = remote::get_current_branch()?;
            if !remote::branch_has_remote(&current)? {
                bail!(
                    "{}",
                    catalog.tf("error.branch.no_remote", &[("branch", current.to_string())])
                );
            }
            current
        }
    };

    let forced_remote = sub.get_one::<String>("remote").cloned();
    let remote = if let Some(value) = forced_remote {
        value
    } else {
        remote::detect_remote_for_branch(&branch)?
    };

    let force_gerrit = sub.get_flag("gerrit");
    let is_gerrit = if force_gerrit {
        true
    } else {
        remote::is_remote_gerrit(&remote)?
    };
    let refspec = if is_gerrit {
        format!("HEAD:refs/for/{branch}")
    } else {
        format!("HEAD:{branch}")
    };

    let mut args: Vec<String> = vec!["push".to_string()];
    if sub.get_flag("no-thin") {
        args.push("--no-thin".to_string());
    }
    if sub.get_flag("force-with-lease") {
        args.push("--force-with-lease".to_string());
    }
    args.push(remote);
    args.push(refspec);

    let dry_run = sub.get_flag("dry-run");
    let verbose = sub.get_flag("verbose");
    if dry_run || verbose {
        println!(
            "{}",
            catalog.tf("status.command.preview", &[("args", args.join(" "))])
        );
    }

    if !dry_run {
        let status = run_git_cmd(&args)?;
        if !status.success() {
            bail!(
                "{}",
                catalog.tf("error.push.failed", &[("status", status.to_string())])
            );
        }
    }

    Ok(())
}

fn execute_setup(sub: &ArgMatches, catalog: &Catalog, runtime: &RuntimeConfig) -> Result<()> {
    let project_scope = sub.get_flag("project");
    let target = if project_scope {
        let root = runtime
            .git_root
            .clone()
            .ok_or_else(|| anyhow::anyhow!(catalog.t("error.not_in_git_workspace")))?;
        let path = runtime
            .project_path
            .clone()
            .unwrap_or_else(|| config::project_config_path(&root));
        println!(
            "{}",
            catalog.tf(
                "status.setup.scope.project",
                &[("root", root.display().to_string())]
            )
        );
        path
    } else {
        println!("{}", catalog.t("status.setup.scope.global"));
        runtime.global_path.clone()
    };

    config::ensure_config_parent(&target)?;
    println!(
        "{}",
        catalog.tf(
            "status.setup.target",
            &[("path", target.display().to_string())]
        )
    );
    setup_ui::run_setup_ui(catalog, &runtime.effective, &target)?;
    Ok(())
}

fn execute_annotate(
    sub: &ArgMatches,
    catalog: &Catalog,
    runtime: &RuntimeConfig,
    cwd: &std::path::Path,
) -> Result<()> {
    if !runtime.effective.features.annotate {
        bail!(
            "{}",
            catalog.tf(
                "error.feature.disabled",
                &[("feature", catalog.t("feature.annotate"))]
            )
        );
    }
    let options = annotate::AnnotateOptions {
        latest_commit: sub.get_flag("latest-commit"),
        include_untracked_override: if sub.get_flag("include-untracked") {
            Some(true)
        } else {
            None
        },
        reason: sub.get_one::<String>("reason").cloned(),
        reference_kind: sub.get_one::<String>("reference-kind").cloned(),
        reference_value: sub.get_one::<String>("reference-value").cloned(),
    };
    annotate::run(options, &runtime.effective, catalog, cwd)
}

fn execute_reset(sub: &ArgMatches, catalog: &Catalog) -> Result<()> {
    if which::which("git").is_err() {
        bail!("{}", catalog.t("error.git.not_found"));
    }

    let current_branch = remote::get_checked_out_local_branch()?
        .ok_or_else(|| anyhow::anyhow!(catalog.t("error.reset.detached_head")))?;
    let upstream = remote::get_upstream_remote_branch(&current_branch)?.ok_or_else(|| {
        anyhow::anyhow!(
            catalog.tf("error.reset.no_upstream", &[("branch", current_branch.clone())])
        )
    })?;

    let mut args: Vec<String> = vec!["reset".to_string()];
    if sub.get_flag("hard") {
        args.push("--hard".to_string());
    }
    args.push(upstream);

    let status = run_git_cmd(&args)?;
    if !status.success() {
        bail!(
            "{}",
            catalog.tf("error.reset.failed", &[("status", status.to_string())])
        );
    }
    Ok(())
}

fn execute_checkout_remote(sub: &ArgMatches, catalog: &Catalog) -> Result<()> {
    if which::which("git").is_err() {
        bail!("{}", catalog.t("error.git.not_found"));
    }

    let remote_branch = sub
        .get_one::<String>("remote-branch")
        .expect("required by clap")
        .to_string();
    let local_branch = sub
        .get_one::<String>("local-branch")
        .cloned()
        .unwrap_or_else(|| remote_branch.clone());

    if remote::local_branch_exists(&local_branch)? {
        bail!(
            "{}",
            catalog.tf(
                "error.checkout_remote.local_exists",
                &[("branch", local_branch.clone())]
            )
        );
    }

    let target = if let Some(preferred_remote) = remote::detect_preferred_remote()? {
        if remote::remote_tracking_branch_exists(&preferred_remote, &remote_branch)? {
            format!("{preferred_remote}/{remote_branch}")
        } else {
            resolve_checkout_remote_target(catalog, &remote_branch)?
        }
    } else {
        resolve_checkout_remote_target(catalog, &remote_branch)?
    };

    let args = vec![
        "checkout".to_string(),
        "-b".to_string(),
        local_branch,
        target,
    ];
    let status = run_git_cmd(&args)?;
    if !status.success() {
        bail!(
            "{}",
            catalog.tf("error.checkout_remote.failed", &[("status", status.to_string())])
        );
    }
    Ok(())
}

fn execute_completion(sub: &ArgMatches, catalog: &Catalog, runtime: &RuntimeConfig) -> Result<()> {
    if sub.get_flag("install") {
        return execute_completion_install(sub, catalog, runtime);
    }

    let shell = sub
        .get_one::<String>("shell")
        .expect("required by clap");
    let script = generate_completion_script(shell, catalog, runtime)?;
    print!("{script}");
    Ok(())
}

fn execute_completion_install(
    sub: &ArgMatches,
    catalog: &Catalog,
    runtime: &RuntimeConfig,
) -> Result<()> {
    let shell = detect_current_shell()
        .or_else(|| sub.get_one::<String>("shell").map(|value| value.to_ascii_lowercase()))
        .ok_or_else(|| anyhow!("{}", catalog.t("error.completion.detect_shell_failed")))?;
    let home = dirs::home_dir().ok_or_else(|| anyhow!("{}", catalog.t("error.completion.detect_shell_failed")))?;
    let plan = build_completion_install_plan(&shell, &home)?;
    let script = generate_completion_script(&plan.shell, catalog, runtime)?;
    let temp_script_path = write_completion_temp_script(&plan.shell, &script)?;

    for line in build_completion_install_preview_lines(catalog, &plan, &temp_script_path) {
        println!("{line}");
    }

    let confirmed = prompt_completion_install_confirm(catalog)?;
    let installed = finalize_completion_install(&plan, &script, confirmed)?;
    if installed {
        println!(
            "{}",
            catalog.tf(
                "status.completion.install.done",
                &[("shell", plan.shell.clone())]
            )
        );
    } else {
        println!("{}", catalog.t("status.completion.install.cancelled"));
    }
    Ok(())
}

fn generate_completion_script(
    shell: &str,
    catalog: &Catalog,
    runtime: &RuntimeConfig,
) -> Result<String> {
    let normalized_shell = shell.to_lowercase();
    let mut command = build_runtime_command(catalog, runtime);
    let mut buffer = Vec::<u8>::new();
    match normalized_shell.as_str() {
        "bash" => generate(Bash, &mut command, "xgit", &mut buffer),
        "zsh" => generate(Zsh, &mut command, "xgit", &mut buffer),
        "fish" => generate(Fish, &mut command, "xgit", &mut buffer),
        "powershell" | "pwsh" => generate(PowerShell, &mut command, "xgit", &mut buffer),
        _ => {
            bail!(
                "{}",
                catalog.tf(
                    "error.completion.unsupported_shell",
                    &[("shell", normalized_shell)]
                )
            );
        }
    }
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

fn build_completion_install_plan(shell: &str, home: &Path) -> Result<CompletionInstallPlan> {
    let normalized = shell.to_ascii_lowercase();
    match normalized.as_str() {
        "zsh" => Ok(CompletionInstallPlan {
            shell: "zsh".to_string(),
            script_target_path: home.join(".xgit").join("completions").join("_xgit"),
            profile_path: Some(home.join(".zshrc")),
            profile_lines: vec![
                "fpath=(~/.xgit/completions $fpath)".to_string(),
                "autoload -U compinit && compinit".to_string(),
            ],
        }),
        "bash" => Ok(CompletionInstallPlan {
            shell: "bash".to_string(),
            script_target_path: home.join(".xgit").join("completions").join("xgit.bash"),
            profile_path: Some(preferred_bash_profile_path(home)),
            profile_lines: vec!["source ~/.xgit/completions/xgit.bash".to_string()],
        }),
        "fish" => Ok(CompletionInstallPlan {
            shell: "fish".to_string(),
            script_target_path: home
                .join(".config")
                .join("fish")
                .join("completions")
                .join("xgit.fish"),
            profile_path: None,
            profile_lines: Vec::new(),
        }),
        "powershell" | "pwsh" => Ok(CompletionInstallPlan {
            shell: "powershell".to_string(),
            script_target_path: home.join(".xgit").join("completions").join("xgit.ps1"),
            profile_path: Some(
                home.join(".config")
                    .join("powershell")
                    .join("Microsoft.PowerShell_profile.ps1"),
            ),
            profile_lines: vec![". \"$HOME/.xgit/completions/xgit.ps1\"".to_string()],
        }),
        _ => bail!("unsupported install shell: {normalized}"),
    }
}

fn preferred_bash_profile_path(home: &Path) -> PathBuf {
    for candidate in [".bashrc", ".bash_profile", ".profile"] {
        let path = home.join(candidate);
        if path.exists() {
            return path;
        }
    }
    home.join(".bashrc")
}

fn detect_current_shell() -> Option<String> {
    let shell_path = std::env::var("SHELL").ok()?;
    infer_shell_from_path(&shell_path)
}

fn infer_shell_from_path(shell_path: &str) -> Option<String> {
    let shell_name = Path::new(shell_path)
        .file_name()?
        .to_string_lossy()
        .to_ascii_lowercase();
    match shell_name.as_str() {
        "zsh" => Some("zsh".to_string()),
        "bash" => Some("bash".to_string()),
        "fish" => Some("fish".to_string()),
        "pwsh" | "powershell" => Some("powershell".to_string()),
        _ => None,
    }
}

fn write_completion_temp_script(shell: &str, script: &str) -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xgit-completion-{shell}-{ts}.tmp"));
    fs::write(&path, script)?;
    Ok(path)
}

fn build_completion_install_preview_lines(
    catalog: &Catalog,
    plan: &CompletionInstallPlan,
    temp_script_path: &Path,
) -> Vec<String> {
    let mut lines = vec![
        catalog.tf(
            "status.completion.install.detected_shell",
            &[("shell", plan.shell.clone())],
        ),
        catalog.tf(
            "status.completion.install.temp_script",
            &[("path", temp_script_path.display().to_string())],
        ),
        catalog.tf(
            "status.completion.install.target_script",
            &[("path", plan.script_target_path.display().to_string())],
        ),
    ];

    if let Some(profile_path) = &plan.profile_path {
        lines.push(catalog.tf(
            "status.completion.install.target_profile",
            &[("path", profile_path.display().to_string())],
        ));
        for line in &plan.profile_lines {
            lines.push(
                catalog.tf("status.completion.install.managed_line", &[("line", line.clone())]),
            );
        }
    } else {
        lines.push(catalog.t("status.completion.install.target_profile_none"));
    }
    lines
}

fn prompt_completion_install_confirm(catalog: &Catalog) -> Result<bool> {
    print!("{} [y/N]: ", catalog.t("status.completion.install.confirm"));
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "Y" | "y"))
}

fn finalize_completion_install(
    plan: &CompletionInstallPlan,
    script: &str,
    confirmed: bool,
) -> Result<bool> {
    if !confirmed {
        return Ok(false);
    }
    install_completion_artifacts(plan, script)?;
    Ok(true)
}

fn install_completion_artifacts(plan: &CompletionInstallPlan, script: &str) -> Result<()> {
    if let Some(parent) = plan.script_target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&plan.script_target_path, script)?;

    if let Some(profile_path) = &plan.profile_path {
        let profile_block = build_managed_profile_block(&plan.profile_lines);
        if let Some(parent) = profile_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let existing = fs::read_to_string(profile_path).unwrap_or_default();
        let (next_content, changed) = replace_or_append_managed_block(&existing, &profile_block);
        if changed {
            fs::write(profile_path, next_content)?;
        }
    }
    Ok(())
}

fn build_managed_profile_block(profile_lines: &[String]) -> String {
    let mut lines = Vec::<String>::new();
    lines.push(PROFILE_BLOCK_BEGIN.to_string());
    lines.extend(profile_lines.iter().cloned());
    lines.push(PROFILE_BLOCK_END.to_string());
    lines.join("\n")
}

fn replace_or_append_managed_block(content: &str, managed_block: &str) -> (String, bool) {
    let mut normalized_block = managed_block.to_string();
    if !normalized_block.ends_with('\n') {
        normalized_block.push('\n');
    }

    if let Some(start) = content.find(PROFILE_BLOCK_BEGIN) {
        if let Some(end_rel) = content[start..].find(PROFILE_BLOCK_END) {
            let end = start + end_rel + PROFILE_BLOCK_END.len();
            let mut next = String::new();
            next.push_str(&content[..start]);
            if !next.is_empty() && !next.ends_with('\n') {
                next.push('\n');
            }
            next.push_str(&normalized_block);
            let mut tail = content[end..].to_string();
            while tail.starts_with('\n') && next.ends_with('\n') {
                tail.remove(0);
            }
            next.push_str(&tail);
            let changed = next != content;
            return (next, changed);
        }
    }

    let mut next = content.to_string();
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(&normalized_block);
    let changed = next != content;
    (next, changed)
}

fn resolve_checkout_remote_target(catalog: &Catalog, remote_branch: &str) -> Result<String> {
    let candidates = remote::list_remote_branch_candidates(remote_branch)?;
    if candidates.is_empty() {
        bail!(
            "{}",
            catalog.tf(
                "error.checkout_remote.remote_not_found",
                &[("branch", remote_branch.to_string())]
            )
        );
    }
    if candidates.len() > 1 {
        bail!(
            "{}",
            catalog.tf(
                "error.checkout_remote.remote_ambiguous",
                &[
                    ("branch", remote_branch.to_string()),
                    ("candidates", candidates.join(", "))
                ]
            )
        );
    }
    Ok(candidates[0].clone())
}

#[cfg(test)]
mod tests {
    use super::{
        build_completion_install_plan, build_completion_install_preview_lines,
        build_managed_profile_block, build_runtime_command, finalize_completion_install,
        generate_completion_script, infer_shell_from_path, replace_or_append_managed_block,
        PROFILE_BLOCK_BEGIN,
    };
    use crate::config::{AppConfig, FeaturesConfig, RuntimeConfig, UiConfig};
    use crate::i18n;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn help_shows_disabled_status_for_feature_commands() {
        let cwd = std::env::current_dir().unwrap();
        let catalog = i18n::load_catalog("en-US", &cwd).unwrap();
        let runtime = RuntimeConfig {
            effective: AppConfig {
                ui: UiConfig {
                    lang: "en-US".to_string(),
                },
                features: FeaturesConfig {
                    push: false,
                    annotate: false,
                },
                annotate: crate::config::AnnotateConfig::default(),
                ..AppConfig::default()
            },
            global_path: PathBuf::from("/tmp/.xgit/config.toml"),
            project_path: None,
            git_root: None,
        };

        let mut cmd = build_runtime_command(&catalog, &runtime);
        let rendered = cmd.render_help().to_string();
        assert!(rendered.contains("(disabled)"));
        assert!(rendered.contains("setup"));
        assert!(!rendered.contains("--lang"));
    }

    #[test]
    fn help_lists_remote_branch_operations() {
        let cwd = std::env::current_dir().unwrap();
        let catalog = i18n::load_catalog("en-US", &cwd).unwrap();
        let runtime = RuntimeConfig {
            effective: AppConfig::default(),
            global_path: PathBuf::from("/tmp/.xgit/config.toml"),
            project_path: None,
            git_root: None,
        };

        let mut cmd = build_runtime_command(&catalog, &runtime);
        let rendered = cmd.render_help().to_string();
        assert!(rendered.contains("reset"));
        assert!(rendered.contains("checkout-remote"));
    }

    #[test]
    fn help_lists_completion_command() {
        let cwd = std::env::current_dir().unwrap();
        let catalog = i18n::load_catalog("en-US", &cwd).unwrap();
        let runtime = RuntimeConfig {
            effective: AppConfig::default(),
            global_path: PathBuf::from("/tmp/.xgit/config.toml"),
            project_path: None,
            git_root: None,
        };

        let mut cmd = build_runtime_command(&catalog, &runtime);
        let rendered = cmd.render_help().to_string();
        assert!(rendered.contains("completion"));

        let completion_help = cmd
            .find_subcommand_mut("completion")
            .expect("completion subcommand must exist")
            .render_help()
            .to_string();
        assert!(completion_help.contains("SHELL"));
    }

    #[test]
    fn completion_script_generated_for_supported_shell() {
        let cwd = std::env::current_dir().unwrap();
        let catalog = i18n::load_catalog("en-US", &cwd).unwrap();
        let runtime = RuntimeConfig {
            effective: AppConfig::default(),
            global_path: PathBuf::from("/tmp/.xgit/config.toml"),
            project_path: None,
            git_root: None,
        };

        let script = generate_completion_script("bash", &catalog, &runtime).unwrap();
        assert!(script.contains("_xgit"));
        assert!(script.contains("complete -F"));
    }

    #[test]
    fn completion_script_fails_for_unknown_shell() {
        let cwd = std::env::current_dir().unwrap();
        let catalog = i18n::load_catalog("en-US", &cwd).unwrap();
        let runtime = RuntimeConfig {
            effective: AppConfig::default(),
            global_path: PathBuf::from("/tmp/.xgit/config.toml"),
            project_path: None,
            git_root: None,
        };

        let err = generate_completion_script("tcsh", &catalog, &runtime).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("unsupported shell"));
    }

    #[test]
    fn completion_install_plan_for_zsh_points_to_expected_targets() {
        let home = TempDir::new().unwrap();
        let plan = build_completion_install_plan("zsh", home.path()).unwrap();
        assert_eq!(
            plan.script_target_path,
            home.path().join(".xgit").join("completions").join("_xgit")
        );
        assert_eq!(plan.profile_path, Some(home.path().join(".zshrc")));
        assert!(plan
            .profile_lines
            .iter()
            .any(|line| line.contains("compinit")));
    }

    #[test]
    fn completion_install_preview_includes_target_paths() {
        let home = TempDir::new().unwrap();
        let plan = build_completion_install_plan("bash", home.path()).unwrap();
        let catalog = i18n::load_catalog("en-US", home.path()).unwrap();
        let temp_script = home.path().join("xgit-completion-preview.tmp");
        let lines = build_completion_install_preview_lines(&catalog, &plan, &temp_script);
        let merged = lines.join("\n");
        assert!(merged.contains(temp_script.to_string_lossy().as_ref()));
        assert!(merged.contains(plan.script_target_path.to_string_lossy().as_ref()));
        assert!(merged.contains(
            plan.profile_path
                .as_ref()
                .unwrap()
                .to_string_lossy()
                .as_ref()
        ));
    }

    #[test]
    fn completion_install_does_not_write_when_not_confirmed() {
        let home = TempDir::new().unwrap();
        let plan = build_completion_install_plan("bash", home.path()).unwrap();
        let installed = finalize_completion_install(&plan, "demo-script", false).unwrap();
        assert!(!installed);
        assert!(!plan.script_target_path.exists());
        if let Some(profile_path) = &plan.profile_path {
            assert!(!profile_path.exists());
        }
    }

    #[test]
    fn completion_install_writes_when_confirmed() {
        let home = TempDir::new().unwrap();
        let plan = build_completion_install_plan("bash", home.path()).unwrap();
        let installed = finalize_completion_install(&plan, "demo-script", true).unwrap();
        assert!(installed);
        assert!(plan.script_target_path.exists());
        assert_eq!(fs::read_to_string(&plan.script_target_path).unwrap(), "demo-script");
        let profile = fs::read_to_string(plan.profile_path.as_ref().unwrap()).unwrap();
        assert!(profile.contains(PROFILE_BLOCK_BEGIN));
        assert!(profile.contains("source ~/.xgit/completions/xgit.bash"));
    }

    #[test]
    fn managed_profile_block_is_replaced_instead_of_appended() {
        let first_block = build_managed_profile_block(&["source ~/.xgit/completions/xgit.bash".to_string()]);
        let second_block = build_managed_profile_block(&["fpath=(~/.xgit/completions $fpath)".to_string()]);
        let (first_write, _) = replace_or_append_managed_block("", &first_block);
        let (second_write, _) = replace_or_append_managed_block(&first_write, &second_block);
        assert_eq!(second_write.matches(PROFILE_BLOCK_BEGIN).count(), 1);
        assert!(second_write.contains("fpath=(~/.xgit/completions $fpath)"));
        assert!(!second_write.contains("source ~/.xgit/completions/xgit.bash"));
    }

    #[test]
    fn infer_shell_from_path_extracts_supported_shell() {
        assert_eq!(infer_shell_from_path("/bin/zsh"), Some("zsh".to_string()));
        assert_eq!(infer_shell_from_path("/usr/bin/bash"), Some("bash".to_string()));
        assert_eq!(infer_shell_from_path("/opt/homebrew/bin/fish"), Some("fish".to_string()));
        assert_eq!(
            infer_shell_from_path("/usr/local/bin/pwsh"),
            Some("powershell".to_string())
        );
    }
}
