use crate::config::{AppConfig, FileRuleConfig};
use crate::i18n::Catalog;
use anyhow::{anyhow, bail, Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct AnnotateOptions {
    pub latest_commit: bool,
    pub include_untracked_override: Option<bool>,
    pub reason: Option<String>,
    pub reference_kind: Option<String>,
    pub reference_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChangeKind {
    Add,
    Modify,
    Delete,
}

impl ChangeKind {
    fn key(&self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Modify => "modify",
            Self::Delete => "del",
        }
    }
}

#[derive(Debug, Clone)]
struct FileChange {
    path: String,
    kind: ChangeKind,
    from_untracked: bool,
}

#[derive(Debug, Clone)]
struct RuntimeContext {
    reason: String,
    reference_kind: String,
    reference_value: String,
    author_tag: String,
    date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HunkSegment {
    start_line: usize,
    kind: ChangeKind,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

pub fn run(
    options: AnnotateOptions,
    config: &AppConfig,
    catalog: &Catalog,
    cwd: &Path,
) -> Result<()> {
    if which::which("git").is_err() {
        bail!("{}", catalog.t("error.git.not_found"));
    }

    let repo_root = resolve_git_root(cwd)?;
    let context = collect_runtime_context(&options, config, catalog, &repo_root)?;
    let changes = if options.latest_commit {
        collect_latest_commit_changes(&repo_root, catalog)?
    } else {
        let include_untracked = options
            .include_untracked_override
            .unwrap_or(config.annotate.staged.include_untracked);
        collect_staged_changes(&repo_root, include_untracked)?
    };

    if changes.is_empty() {
        println!("{}", catalog.t("status.annotate.no_changes"));
        return Ok(());
    }

    let mut applied = 0usize;
    for change in &changes {
        let renderer = select_renderer(&change.path, &config.annotate.file_rules);
        match renderer.as_deref() {
            Some("c_line_block") => {
                if !options.latest_commit
                    && !change.from_untracked
                    && has_unstaged_changes_for_path(&repo_root, &change.path)?
                {
                    bail!(
                        "{}",
                        catalog.tf(
                            "error.annotate.staged_unstaged_conflict",
                            &[("path", change.path.clone())]
                        )
                    );
                }

                let Some(target_content) =
                    load_target_content(&repo_root, options.latest_commit, change)?
                else {
                    println!(
                        "{}",
                        catalog.tf("status.annotate.no_rule", &[("path", change.path.clone())])
                    );
                    continue;
                };

                let segments = collect_hunk_segments(
                    &repo_root,
                    options.latest_commit,
                    change,
                    &target_content,
                )?;
                if segments.is_empty() {
                    continue;
                }

                let updated = apply_c_line_segments(
                    &target_content,
                    &segments,
                    &context,
                    config,
                    &change.path,
                );
                write_output_file(&repo_root, &change.path, &updated)?;
                applied += 1;
            }
            Some(other) => {
                println!(
                    "{}",
                    catalog.tf(
                        "status.annotate.renderer_unimplemented",
                        &[
                            ("path", change.path.clone()),
                            ("renderer", other.to_string()),
                        ],
                    )
                );
            }
            None => {
                println!(
                    "{}",
                    catalog.tf("status.annotate.no_rule", &[("path", change.path.clone())])
                );
            }
        }
    }

    if options.latest_commit {
        println!("{}", catalog.t("status.annotate.latest_commit_hint"));
    }
    println!(
        "{}",
        catalog.tf(
            "status.annotate.summary",
            &[
                ("count", applied.to_string()),
                ("total", changes.len().to_string())
            ],
        )
    );
    Ok(())
}

fn resolve_git_root(cwd: &Path) -> Result<PathBuf> {
    let root = git_stdout(cwd, &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(root.trim()))
}

fn collect_runtime_context(
    options: &AnnotateOptions,
    config: &AppConfig,
    catalog: &Catalog,
    cwd: &Path,
) -> Result<RuntimeContext> {
    let fields = &config.annotate.form.fields;
    let mut reason = options.reason.clone().unwrap_or_default();
    let mut reference_kind = options.reference_kind.clone().unwrap_or_default();
    let mut reference_value = options.reference_value.clone().unwrap_or_default();

    if reason.is_empty() && fields.iter().any(|f| f == "reason") {
        reason = prompt_line(&catalog.t("prompt.annotate.reason"))?;
    }

    if reference_kind.is_empty() && fields.iter().any(|f| f == "reference_kind") {
        let prompt = format!(
            "{} [{}]",
            catalog.t("prompt.annotate.reference_kind"),
            config.annotate.reference_kinds.join(",")
        );
        reference_kind = prompt_line(&prompt)?;
    }
    if reference_kind.is_empty() && !config.annotate.reference_kinds.is_empty() {
        reference_kind = config.annotate.reference_kinds[0].clone();
    }

    if !config.annotate.reference_kinds.is_empty()
        && !config
            .annotate
            .reference_kinds
            .iter()
            .any(|k| k == &reference_kind)
    {
        bail!(
            "{}",
            catalog.tf(
                "error.annotate.reference_kind_invalid",
                &[("kind", reference_kind.clone())]
            )
        );
    }

    if reference_value.is_empty() && fields.iter().any(|f| f == "reference_value") {
        reference_value = prompt_line(&catalog.t("prompt.annotate.reference_value"))?;
    }

    Ok(RuntimeContext {
        reason,
        reference_kind,
        reference_value,
        author_tag: resolve_author_tag(cwd, config),
        date: current_timestamp_tag(),
    })
}

fn resolve_author_tag(cwd: &Path, config: &AppConfig) -> String {
    if let Some(author_tag) = config.identity.author_tag.clone() {
        let trimmed = author_tag.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }

    if let Ok(name) = git_stdout(cwd, &["config", "--get", "user.name"]) {
        let trimmed = name.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    "unknown".to_string()
}

fn current_timestamp_tag() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}

fn prompt_line(prompt: &str) -> Result<String> {
    print!("{prompt}: ");
    io::stdout().flush()?;
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer.trim().to_string())
}

fn collect_staged_changes(cwd: &Path, include_untracked: bool) -> Result<Vec<FileChange>> {
    let mut changes =
        parse_name_status_output(&git_stdout(cwd, &["diff", "--staged", "--name-status"])?);

    if include_untracked {
        let untracked = git_stdout(cwd, &["ls-files", "--others", "--exclude-standard"])?;
        for file in untracked.lines().filter(|s| !s.trim().is_empty()) {
            changes.push(FileChange {
                path: file.trim().to_string(),
                kind: ChangeKind::Add,
                from_untracked: true,
            });
        }
    }
    dedup_changes(changes)
}

fn collect_latest_commit_changes(cwd: &Path, catalog: &Catalog) -> Result<Vec<FileChange>> {
    validate_latest_commit_mode(cwd, catalog)?;
    let output = git_stdout(cwd, &["diff", "--name-status", "HEAD^", "HEAD"])?;
    dedup_changes(parse_name_status_output(&output))
}

fn validate_latest_commit_mode(cwd: &Path, catalog: &Catalog) -> Result<()> {
    let parent = Command::new("git")
        .current_dir(cwd)
        .args(["rev-parse", "--verify", "HEAD^"])
        .output()?;
    if !parent.status.success() {
        bail!("{}", catalog.t("error.annotate.latest_commit_root"));
    }

    let parents = git_stdout(cwd, &["rev-list", "--parents", "-n", "1", "HEAD"])?;
    if parents.split_whitespace().count() > 2 {
        bail!("{}", catalog.t("error.annotate.latest_commit_merge"));
    }

    let status = git_stdout(cwd, &["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        bail!("{}", catalog.t("error.annotate.latest_commit_dirty"));
    }

    Ok(())
}

fn dedup_changes(changes: Vec<FileChange>) -> Result<Vec<FileChange>> {
    let mut map: BTreeMap<String, FileChange> = BTreeMap::new();
    for change in changes {
        if let Some(existing) = map.get(&change.path) {
            if !existing.from_untracked {
                continue;
            }
        }
        map.insert(change.path.clone(), change);
    }
    Ok(map.into_values().collect())
}

fn parse_name_status_output(output: &str) -> Vec<FileChange> {
    let mut out = Vec::new();
    for line in output.lines().filter(|s| !s.trim().is_empty()) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.is_empty() {
            continue;
        }
        let status = cols[0];
        let path = if status.starts_with('R') || status.starts_with('C') {
            cols.last().copied().unwrap_or_default()
        } else {
            cols.get(1).copied().unwrap_or_default()
        };
        if path.is_empty() {
            continue;
        }
        let kind = match status.chars().next().unwrap_or('M') {
            'A' => ChangeKind::Add,
            'D' => ChangeKind::Delete,
            _ => ChangeKind::Modify,
        };
        out.push(FileChange {
            path: path.to_string(),
            kind,
            from_untracked: false,
        });
    }
    out
}

fn load_target_content(
    cwd: &Path,
    latest_commit: bool,
    change: &FileChange,
) -> Result<Option<String>> {
    if latest_commit {
        if change.kind == ChangeKind::Delete {
            return Ok(None);
        }
        return git_show_content(cwd, &format!("HEAD:{}", change.path));
    }

    if change.from_untracked {
        return fs::read_to_string(cwd.join(&change.path))
            .map(Some)
            .with_context(|| format!("failed to read untracked file {}", change.path));
    }

    if change.kind == ChangeKind::Delete {
        return Ok(None);
    }
    git_show_content(cwd, &format!(":{}", change.path))
}

fn git_show_content(cwd: &Path, spec: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(["show", spec])
        .output()
        .with_context(|| format!("failed to run git show {spec}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}

fn collect_hunk_segments(
    cwd: &Path,
    latest_commit: bool,
    change: &FileChange,
    current_content: &str,
) -> Result<Vec<HunkSegment>> {
    if change.from_untracked {
        let lines = current_content
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        if lines.is_empty() {
            return Ok(vec![]);
        }
        return Ok(vec![HunkSegment {
            start_line: 1,
            kind: ChangeKind::Add,
            old_lines: vec![],
            new_lines: lines,
        }]);
    }

    let patch = diff_patch_for_file(cwd, latest_commit, &change.path)?;
    Ok(parse_hunk_segments(&patch))
}

fn parse_hunk_segments(patch: &str) -> Vec<HunkSegment> {
    let mut segments = Vec::new();
    let mut in_hunk = false;
    let mut current_new_line = 1usize;
    let mut pending_start: Option<usize> = None;
    let mut pending_old: Vec<String> = Vec::new();
    let mut pending_new: Vec<String> = Vec::new();

    for line in patch.lines() {
        if line.starts_with("@@") {
            push_pending_segment(
                &mut segments,
                &mut pending_start,
                &mut pending_old,
                &mut pending_new,
                current_new_line,
            );
            in_hunk = true;
            current_new_line = parse_hunk_new_start(line).unwrap_or(1);
            continue;
        }
        if !in_hunk {
            continue;
        }

        match line.chars().next() {
            Some(' ') => {
                push_pending_segment(
                    &mut segments,
                    &mut pending_start,
                    &mut pending_old,
                    &mut pending_new,
                    current_new_line,
                );
                current_new_line += 1;
            }
            Some('-') => {
                if pending_start.is_none() {
                    pending_start = Some(current_new_line);
                }
                pending_old.push(line[1..].to_string());
            }
            Some('+') => {
                if pending_start.is_none() {
                    pending_start = Some(current_new_line);
                }
                pending_new.push(line[1..].to_string());
                current_new_line += 1;
            }
            _ => {}
        }
    }

    push_pending_segment(
        &mut segments,
        &mut pending_start,
        &mut pending_old,
        &mut pending_new,
        current_new_line,
    );
    segments
}

fn push_pending_segment(
    segments: &mut Vec<HunkSegment>,
    pending_start: &mut Option<usize>,
    pending_old: &mut Vec<String>,
    pending_new: &mut Vec<String>,
    fallback_line: usize,
) {
    if pending_old.is_empty() && pending_new.is_empty() {
        return;
    }

    let kind = if pending_old.is_empty() {
        ChangeKind::Add
    } else if pending_new.is_empty() {
        ChangeKind::Delete
    } else {
        ChangeKind::Modify
    };
    let start_line = pending_start.unwrap_or(fallback_line).max(1);

    segments.push(HunkSegment {
        start_line,
        kind,
        old_lines: std::mem::take(pending_old),
        new_lines: std::mem::take(pending_new),
    });
    *pending_start = None;
}

fn parse_hunk_new_start(header: &str) -> Option<usize> {
    let mut parts = header.split_whitespace();
    parts.next()?;
    let _old = parts.next()?;
    let new_range = parts.next()?;
    parse_range_start(new_range, '+')
}

fn parse_range_start(token: &str, marker: char) -> Option<usize> {
    token
        .strip_prefix(marker)?
        .split(',')
        .next()?
        .parse::<usize>()
        .ok()
}

fn apply_c_line_segments(
    content: &str,
    segments: &[HunkSegment],
    context: &RuntimeContext,
    config: &AppConfig,
    path: &str,
) -> String {
    let trailing_newline = content.ends_with('\n');
    let mut lines = content
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    for segment in segments.iter().rev() {
        let (prefix, suffix) = render_c_line_block(segment, context, config, path);
        let index = segment.start_line.saturating_sub(1).min(lines.len());
        lines.splice(index..index, prefix.clone());

        let suffix_index = if segment.kind == ChangeKind::Delete {
            index + prefix.len()
        } else {
            (index + prefix.len() + segment.new_lines.len()).min(lines.len())
        };
        lines.splice(suffix_index..suffix_index, suffix.clone());
    }

    if lines.is_empty() {
        return String::new();
    }

    let mut output = lines.join("\n");
    if trailing_newline {
        output.push('\n');
    }
    output
}

fn render_c_line_block(
    segment: &HunkSegment,
    context: &RuntimeContext,
    config: &AppConfig,
    path: &str,
) -> (Vec<String>, Vec<String>) {
    let template = match segment.kind {
        ChangeKind::Add => &config.annotate.policies.add,
        ChangeKind::Modify => &config.annotate.policies.modify,
        ChangeKind::Delete => &config.annotate.policies.del,
    };
    let rendered = expand_policy_template(
        template,
        context,
        segment.kind.key(),
        path,
        &segment.old_lines,
        &segment.new_lines,
    );

    let mut prefix = rendered.lines().map(c_line_comment).collect::<Vec<_>>();
    if (segment.kind == ChangeKind::Modify || segment.kind == ChangeKind::Delete)
        && !segment.old_lines.is_empty()
        && !template.contains("{old}")
    {
        prefix.push(c_line_comment("old:"));
        for old in &segment.old_lines {
            prefix.push(c_line_comment(&format!("  {old}")));
        }
    }

    let suffix = vec![c_line_comment(&format!("end {}", segment.kind.key()))];
    (prefix, suffix)
}

fn c_line_comment(content: &str) -> String {
    if content.trim().is_empty() {
        "//".to_string()
    } else {
        format!("// {content}")
    }
}

fn expand_policy_template(
    template: &str,
    context: &RuntimeContext,
    kind: &str,
    path: &str,
    old_lines: &[String],
    new_lines: &[String],
) -> String {
    let old = old_lines.join("\\n");
    let new = new_lines.join("\\n");
    let mut rendered = template.to_string();
    for (key, value) in [
        ("{author_tag}", context.author_tag.as_str()),
        ("{date}", context.date.as_str()),
        ("{reason}", context.reason.as_str()),
        ("{reference_kind}", context.reference_kind.as_str()),
        ("{reference_value}", context.reference_value.as_str()),
        ("{kind}", kind),
        ("{path}", path),
        ("{old}", old.as_str()),
        ("{new}", new.as_str()),
    ] {
        rendered = rendered.replace(key, value);
    }
    rendered
}

fn write_output_file(cwd: &Path, file: &str, content: &str) -> Result<()> {
    let path = cwd.join(file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory for {}", path.display()))?;
    }
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn has_unstaged_changes_for_path(cwd: &Path, file: &str) -> Result<bool> {
    let output = git_stdout(cwd, &["diff", "--name-only", "--", file])?;
    Ok(!output.trim().is_empty())
}

fn diff_patch_for_file(cwd: &Path, latest_commit: bool, file: &str) -> Result<String> {
    if latest_commit {
        git_stdout(cwd, &["diff", "--unified=0", "HEAD^", "HEAD", "--", file])
    } else {
        git_stdout(cwd, &["diff", "--staged", "--unified=0", "--", file])
    }
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow!("git {} failed: {}", args.join(" "), stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn select_renderer(path: &str, rules: &[FileRuleConfig]) -> Option<String> {
    for rule in rules {
        if matches_pattern(path, &rule.pattern) {
            return Some(rule.renderer.clone());
        }
    }
    None
}

fn matches_pattern(path: &str, pattern: &str) -> bool {
    let normalized_path = path.replace('\\', "/");
    let normalized_pattern = pattern.replace('\\', "/");
    if let Some(ext) = normalized_pattern.strip_prefix("*.") {
        return normalized_path.ends_with(&format!(".{ext}"));
    }
    normalized_path == normalized_pattern
}

#[cfg(test)]
mod tests {
    use super::{
        apply_c_line_segments, collect_latest_commit_changes, collect_runtime_context,
        collect_staged_changes, git_stdout, matches_pattern, parse_hunk_segments,
        parse_name_status_output, run, ChangeKind, HunkSegment, RuntimeContext,
    };
    use crate::config::AppConfig;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn parse_name_status() {
        let output = "A\tfoo.c\nM\tbar.c\nD\tbaz.c\nR100\told.c\tnew.c\n";
        let parsed = parse_name_status_output(output);
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0].kind, ChangeKind::Add);
        assert_eq!(parsed[1].kind, ChangeKind::Modify);
        assert_eq!(parsed[2].kind, ChangeKind::Delete);
        assert_eq!(parsed[3].path, "new.c");
    }

    #[test]
    fn parse_hunks_with_old_and_new_blocks() {
        let patch = r#"diff --git a/demo.c b/demo.c
index a1b2c3d..e4f5a6b 100644
--- a/demo.c
+++ b/demo.c
@@ -1 +1 @@
-int a = 1;
+int a = 2;
"#;
        let segments = parse_hunk_segments(patch);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].kind, ChangeKind::Modify);
        assert_eq!(segments[0].old_lines, vec!["int a = 1;".to_string()]);
        assert_eq!(segments[0].new_lines, vec!["int a = 2;".to_string()]);
    }

    #[test]
    fn c_like_pattern_match() {
        assert!(matches_pattern("src/main.c", "*.c"));
        assert!(!matches_pattern("src/main.rs", "*.c"));
    }

    #[test]
    fn windows_path_pattern_match() {
        assert!(matches_pattern(r"src\main.c", "*.c"));
        assert!(matches_pattern(r"src\main.c", r"src/main.c"));
    }

    #[test]
    fn apply_segments_wraps_changed_lines() {
        let mut cfg = AppConfig::default();
        cfg.annotate.policies.add =
            "add {author_tag}:{reason}:{reference_kind}:{reference_value}".to_string();
        let content = "int a = 1;\nint b = 2;\n";
        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 2,
                kind: ChangeKind::Add,
                old_lines: vec![],
                new_lines: vec!["int b = 2;".to_string()],
            }],
            &RuntimeContext {
                reason: "why".to_string(),
                reference_kind: "bug".to_string(),
                reference_value: "ID-1".to_string(),
                author_tag: "QA".to_string(),
                date: "123".to_string(),
            },
            &cfg,
            "demo.c",
        );
        assert!(updated.contains("// add QA:why:bug:ID-1"));
        assert!(updated.contains("// end add"));
    }

    #[test]
    fn context_uses_provided_values_without_prompt() {
        let cfg = AppConfig::default();
        let catalog = crate::i18n::load_catalog("en-US", Path::new(".")).unwrap();
        let ctx = collect_runtime_context(
            &super::AnnotateOptions {
                latest_commit: false,
                include_untracked_override: None,
                reason: Some("reason".to_string()),
                reference_kind: Some("bug".to_string()),
                reference_value: Some("ID-1".to_string()),
            },
            &cfg,
            &catalog,
            Path::new("."),
        )
        .unwrap();
        assert_eq!(ctx.reason, "reason");
        assert_eq!(ctx.reference_kind, "bug");
        assert_eq!(ctx.reference_value, "ID-1");
    }

    #[test]
    fn staged_source_respects_include_untracked() {
        let repo = init_test_repo();
        fs::write(repo.path().join("tracked.c"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "tracked.c"]);

        fs::write(repo.path().join("new_untracked.c"), "int b = 2;\n").unwrap();

        let without_untracked = collect_staged_changes(repo.path(), false).unwrap();
        assert!(without_untracked.iter().any(|c| c.path == "tracked.c"));
        assert!(!without_untracked
            .iter()
            .any(|c| c.path == "new_untracked.c"));

        let with_untracked = collect_staged_changes(repo.path(), true).unwrap();
        assert!(with_untracked.iter().any(|c| c.path == "new_untracked.c"));
    }

    #[test]
    fn latest_commit_source_collects_last_commit_changes() {
        let repo = init_test_repo();
        fs::write(repo.path().join("demo.c"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "init"]);

        fs::write(repo.path().join("demo.c"), "int a = 2;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "update"]);

        let catalog = crate::i18n::load_catalog("en-US", repo.path()).unwrap();
        let changes = collect_latest_commit_changes(repo.path(), &catalog).unwrap();
        assert!(changes.iter().any(|c| c.path == "demo.c"));
    }

    #[test]
    fn annotate_staged_writes_file_without_refreshing_index() {
        let repo = init_test_repo();
        fs::write(repo.path().join("demo.c"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "init"]);

        fs::write(repo.path().join("demo.c"), "int a = 2;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);

        let catalog = crate::i18n::load_catalog("en-US", repo.path()).unwrap();
        let mut cfg = AppConfig::default();
        cfg.identity.author_tag = Some("QA".to_string());

        run(
            super::AnnotateOptions {
                latest_commit: false,
                include_untracked_override: None,
                reason: Some("fix".to_string()),
                reference_kind: Some("bug".to_string()),
                reference_value: Some("ID-1".to_string()),
            },
            &cfg,
            &catalog,
            repo.path(),
        )
        .unwrap();

        let file = fs::read_to_string(repo.path().join("demo.c")).unwrap();
        assert!(file.contains("fix"));

        let cached = git_stdout(repo.path(), &["diff", "--cached", "--", "demo.c"]).unwrap();
        assert!(!cached.contains("fix"));
        let unstaged = git_stdout(repo.path(), &["diff", "--", "demo.c"]).unwrap();
        assert!(unstaged.contains("fix"));
    }

    #[test]
    fn annotate_latest_commit_materializes_worktree_result() {
        let repo = init_test_repo();
        fs::write(repo.path().join("demo.c"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "init"]);

        fs::write(repo.path().join("demo.c"), "int a = 2;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "update"]);

        let catalog = crate::i18n::load_catalog("en-US", repo.path()).unwrap();
        let mut cfg = AppConfig::default();
        cfg.identity.author_tag = Some("QA".to_string());

        run(
            super::AnnotateOptions {
                latest_commit: true,
                include_untracked_override: None,
                reason: Some("latest".to_string()),
                reference_kind: Some("req".to_string()),
                reference_value: Some("ID-2".to_string()),
            },
            &cfg,
            &catalog,
            repo.path(),
        )
        .unwrap();

        let file = fs::read_to_string(repo.path().join("demo.c")).unwrap();
        assert!(file.contains("latest"));
        let status = git_stdout(repo.path(), &["status", "--porcelain", "--", "demo.c"]).unwrap();
        assert!(!status.trim().is_empty());
    }

    #[test]
    fn annotate_from_subdirectory_works_for_staged_file() {
        let repo = init_test_repo();
        let sub = repo.path().join("networkmgr").join("routemgr");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("DnsEvent.cpp"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "networkmgr/routemgr/DnsEvent.cpp"]);
        run_git(repo.path(), &["commit", "-m", "init"]);

        fs::write(sub.join("DnsEvent.cpp"), "int a = 2;\n").unwrap();
        run_git(repo.path(), &["add", "networkmgr/routemgr/DnsEvent.cpp"]);

        let catalog = crate::i18n::load_catalog("en-US", &sub).unwrap();
        let mut cfg = AppConfig::default();
        cfg.identity.author_tag = Some("QA".to_string());

        run(
            super::AnnotateOptions {
                latest_commit: false,
                include_untracked_override: None,
                reason: Some("subdir".to_string()),
                reference_kind: Some("req".to_string()),
                reference_value: Some("ID-3".to_string()),
            },
            &cfg,
            &catalog,
            &sub,
        )
        .unwrap();

        let file = fs::read_to_string(sub.join("DnsEvent.cpp")).unwrap();
        assert!(file.contains("subdir"));
    }

    fn init_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        run_git(dir.path(), &["init"]);
        run_git(dir.path(), &["config", "user.email", "xgit@test.local"]);
        run_git(dir.path(), &["config", "user.name", "xgit-test"]);
        dir
    }

    fn run_git(cwd: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {} failed", args.join(" "));
    }
}
