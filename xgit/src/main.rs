mod annotate;
mod config;
mod gitutils;
mod i18n;
mod remote;
mod setup_ui;

use anyhow::{bail, Result};
use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgMatches, Command};
use config::{load_runtime_config, LoadConfigOptions, RuntimeConfig};
use gitutils::run_git_cmd;
use i18n::Catalog;

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

#[cfg(test)]
mod tests {
    use super::build_runtime_command;
    use crate::config::{AppConfig, FeaturesConfig, RuntimeConfig, UiConfig};
    use crate::i18n;
    use std::path::PathBuf;

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
}
