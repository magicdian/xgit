use crate::code_file_types::builtin_entry_for_path;
use crate::config::{
    AnnotateOldCodeBlockCommentConfig, AnnotateOldCodeLineCommentConfig, AnnotateOldCodeLineLayout,
    AnnotateOldCodeMode, AppConfig, FileRuleConfig,
};
use crate::i18n::Catalog;
use anyhow::{anyhow, bail, Context, Result};
use chrono::{Local, NaiveDateTime};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
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

#[derive(Debug, Clone)]
struct PreparedCLineFile {
    change: FileChange,
    logical_content: String,
    segments: Vec<HunkSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum UnformattedReason {
    UnsupportedType,
    BuiltinTypeDisabled,
    RendererUnimplemented(String),
    NoTargetContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnformattedFile {
    path: String,
    reason: UnformattedReason,
}

#[derive(Debug, Clone)]
enum CandidateProcessResult {
    Prepared {
        file: PreparedCLineFile,
        context_candidates: Vec<ContextReuseCandidate>,
    },
    Unformatted(UnformattedFile),
}

#[derive(Debug, Clone, Default)]
struct ContextReuseCandidate {
    reason: Option<String>,
    reference_kind: Option<String>,
    reference_value: Option<String>,
}

#[derive(Debug, Clone)]
struct PendingBlockContextDefaults {
    reason: String,
    reference_kind: String,
    reference_value: String,
}

#[derive(Debug, Clone)]
struct NormalizationResult {
    logical_content: String,
    segments: Vec<HunkSegment>,
    context_candidates: Vec<ContextReuseCandidate>,
}

#[derive(Debug, Clone)]
struct CandidateAnnotationBlock {
    kind: ChangeKind,
    start_line: usize,
    end_line: usize,
    code_start_line: usize,
    code_end_line: usize,
    shell_lines: Vec<usize>,
    context_candidate: ContextReuseCandidate,
}

#[derive(Debug, Clone)]
struct BlockPattern {
    kind: ChangeKind,
    start_contains_old_placeholder: bool,
    start_lines: Vec<PatternLine>,
    end_lines: Vec<PatternLine>,
}

#[derive(Debug, Clone)]
struct PatternLine {
    matcher: Regex,
    capture: Regex,
}

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    let mut prepared = Vec::<PreparedCLineFile>::new();
    let mut context_candidates = Vec::<ContextReuseCandidate>::new();
    let mut unformatted = Vec::<UnformattedFile>::new();

    for change in &changes {
        match process_candidate_change(
            change,
            options.latest_commit,
            config,
            catalog,
            &repo_root,
        )? {
            CandidateProcessResult::Prepared {
                file,
                context_candidates: file_context_candidates,
            } => {
                context_candidates.extend(file_context_candidates);
                prepared.push(file);
            }
            CandidateProcessResult::Unformatted(item) => {
                unformatted.push(item);
            }
        }
    }

    let reuse_defaults = resolve_reusable_context_defaults(&context_candidates);
    let context = collect_runtime_context(
        &options,
        config,
        catalog,
        &repo_root,
        reuse_defaults.as_ref(),
    )?;

    let mut applied = 0usize;
    for file in &prepared {
        if file.segments.is_empty() {
            continue;
        }
        let updated = apply_c_line_segments(
            &file.logical_content,
            &file.segments,
            &context,
            config,
            &file.change.path,
        );
        write_output_file(&repo_root, &file.change.path, &updated)?;
        applied += 1;
    }

    if options.latest_commit {
        println!("{}", catalog.t("status.annotate.latest_commit_hint"));
    }
    for line in build_annotate_report_lines(catalog, applied, &unformatted) {
        println!("{line}");
    }
    Ok(())
}

fn process_candidate_change(
    change: &FileChange,
    latest_commit: bool,
    config: &AppConfig,
    catalog: &Catalog,
    repo_root: &Path,
) -> Result<CandidateProcessResult> {
    let renderer = select_renderer(&change.path, &config.annotate.file_rules);
    match renderer.as_deref() {
        Some("c_line_block") => {
            if !latest_commit
                && !change.from_untracked
                && has_unstaged_changes_for_path(repo_root, &change.path)?
            {
                bail!(
                    "{}",
                    catalog.tf(
                        "error.annotate.staged_unstaged_conflict",
                        &[("path", change.path.clone())]
                    )
                );
            }

            let Some(current_content) = load_target_content(repo_root, latest_commit, change)? else {
                return Ok(CandidateProcessResult::Unformatted(UnformattedFile {
                    path: change.path.clone(),
                    reason: UnformattedReason::NoTargetContent,
                }));
            };

            let baseline_content = load_baseline_content(repo_root, latest_commit, change)?;
            let normalized = normalize_content_before_render(
                &baseline_content,
                &current_content,
                config,
                &change.path,
            )
            .with_context(|| {
                catalog.tf(
                    "error.annotate.normalize_failed",
                    &[("path", change.path.clone())],
                )
            })?;

            Ok(CandidateProcessResult::Prepared {
                file: PreparedCLineFile {
                    change: change.clone(),
                    logical_content: normalized.logical_content,
                    segments: normalized.segments,
                },
                context_candidates: normalized.context_candidates,
            })
        }
        Some(other) => Ok(CandidateProcessResult::Unformatted(UnformattedFile {
            path: change.path.clone(),
            reason: UnformattedReason::RendererUnimplemented(other.to_string()),
        })),
        None => Ok(CandidateProcessResult::Unformatted(UnformattedFile {
            path: change.path.clone(),
            reason: classify_missing_rule_reason(&change.path),
        })),
    }
}

fn classify_missing_rule_reason(path: &str) -> UnformattedReason {
    if builtin_entry_for_path(path).is_some() {
        UnformattedReason::BuiltinTypeDisabled
    } else {
        UnformattedReason::UnsupportedType
    }
}

fn format_unformatted_reason(reason: &UnformattedReason, catalog: &Catalog) -> String {
    match reason {
        UnformattedReason::UnsupportedType => {
            catalog.t("status.annotate.unformatted_reason.unsupported_type")
        }
        UnformattedReason::BuiltinTypeDisabled => {
            catalog.t("status.annotate.unformatted_reason.builtin_type_disabled")
        }
        UnformattedReason::RendererUnimplemented(renderer) => catalog.tf(
            "status.annotate.unformatted_reason.renderer_unimplemented",
            &[("renderer", renderer.clone())],
        ),
        UnformattedReason::NoTargetContent => {
            catalog.t("status.annotate.unformatted_reason.no_target_content")
        }
    }
}

fn build_annotate_report_lines(
    catalog: &Catalog,
    rendered_count: usize,
    unformatted: &[UnformattedFile],
) -> Vec<String> {
    let mut lines = vec![catalog.tf(
        "status.annotate.summary",
        &[
            ("rendered", rendered_count.to_string()),
            ("unformatted", unformatted.len().to_string()),
        ],
    )];
    if unformatted.is_empty() {
        return lines;
    }
    lines.push(catalog.t("status.annotate.unformatted_header"));
    for item in unformatted {
        lines.push(catalog.tf(
            "status.annotate.unformatted_item",
            &[
                ("path", item.path.clone()),
                ("reason", format_unformatted_reason(&item.reason, catalog)),
            ],
        ));
    }
    lines
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
    reuse_defaults: Option<&PendingBlockContextDefaults>,
) -> Result<RuntimeContext> {
    let fields = &config.annotate.form.fields;
    let mut reason = options.reason.clone().unwrap_or_default();
    let mut reference_kind = options.reference_kind.clone().unwrap_or_default();
    let mut reference_value = options.reference_value.clone().unwrap_or_default();
    let mut reused_context = false;

    let should_try_reuse = reason.is_empty()
        && reference_kind.is_empty()
        && reference_value.is_empty()
        && fields.iter().any(|field| {
            field == "reason" || field == "reference_kind" || field == "reference_value"
        });
    if should_try_reuse {
        if let Some(defaults) = reuse_defaults {
            let prompt = catalog.tf(
                "prompt.annotate.reuse_context",
                &[
                    ("reason", defaults.reason.clone()),
                    ("reference_kind", defaults.reference_kind.clone()),
                    ("reference_value", defaults.reference_value.clone()),
                ],
            );
            if prompt_yes_no(&prompt, true)? {
                reason = defaults.reason.clone();
                reference_kind = defaults.reference_kind.clone();
                reference_value = defaults.reference_value.clone();
                reused_context = true;
            }
        }
    }

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
        if reused_context {
            reference_kind.clear();
        } else {
            bail!(
                "{}",
                catalog.tf(
                    "error.annotate.reference_kind_invalid",
                    &[("kind", reference_kind.clone())]
                )
            );
        }
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
        date: current_date_tag(&config.annotate.date.format),
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

fn current_date_tag(format: &str) -> String {
    let now = Local::now().naive_local();
    format_date_tag(format, now)
}

fn format_date_tag(format: &str, value: NaiveDateTime) -> String {
    let chrono_format = to_chrono_date_format(format);
    value.format(&chrono_format).to_string()
}

fn to_chrono_date_format(format: &str) -> String {
    format
        .replace("yyyy", "%Y")
        .replace("yy", "%y")
        .replace("mm", "%m")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("MM", "%M")
}

fn prompt_line(prompt: &str) -> Result<String> {
    print!("{prompt}: ");
    io::stdout().flush()?;
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer.trim().to_string())
}

fn prompt_yes_no(prompt: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    loop {
        print!("{prompt} {suffix}: ");
        io::stdout().flush()?;
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        let answer = buffer.trim().to_ascii_lowercase();
        if answer.is_empty() {
            return Ok(default_yes);
        }
        match answer.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => continue,
        }
    }
}

fn resolve_reusable_context_defaults(
    candidates: &[ContextReuseCandidate],
) -> Option<PendingBlockContextDefaults> {
    if candidates.is_empty() {
        return None;
    }
    let reason = unique_candidate_value(candidates.iter().map(|c| c.reason.as_deref()))?;
    let reference_kind =
        unique_candidate_value(candidates.iter().map(|c| c.reference_kind.as_deref()))?;
    let reference_value =
        unique_candidate_value(candidates.iter().map(|c| c.reference_value.as_deref()))?;
    Some(PendingBlockContextDefaults {
        reason,
        reference_kind,
        reference_value,
    })
}

fn normalize_context_candidate(candidate: &mut ContextReuseCandidate, reference_kinds: &[String]) {
    let Some(reference_kind) = candidate.reference_kind.clone() else {
        return;
    };
    let normalized_kind = reference_kind.trim().to_string();
    if reference_kinds
        .iter()
        .any(|kind| kind.as_str() == normalized_kind.as_str())
    {
        candidate.reference_kind = Some(normalized_kind);
        return;
    }

    let Some(reason) = candidate.reason.clone() else {
        return;
    };
    for expected_kind in reference_kinds {
        let suffix = format!(" {}", expected_kind);
        if normalized_kind.ends_with(&suffix) {
            let moved = normalized_kind[..normalized_kind.len() - suffix.len()]
                .trim()
                .to_string();
            if moved.is_empty() {
                continue;
            }
            candidate.reason = Some(format!("{reason} {moved}").trim().to_string());
            candidate.reference_kind = Some(expected_kind.clone());
            return;
        }
    }
}

fn unique_candidate_value<'a>(values: impl Iterator<Item = Option<&'a str>>) -> Option<String> {
    let mut unique: Option<&str> = None;
    for value in values {
        let value = value?.trim();
        if value.is_empty() {
            return None;
        }
        if let Some(existing) = unique {
            if existing != value {
                return None;
            }
        } else {
            unique = Some(value);
        }
    }
    unique.map(std::string::ToString::to_string)
}

fn load_baseline_content(cwd: &Path, latest_commit: bool, change: &FileChange) -> Result<String> {
    if change.from_untracked || change.kind == ChangeKind::Add {
        return Ok(String::new());
    }

    let spec = if latest_commit {
        format!("HEAD^:{}", change.path)
    } else {
        format!("HEAD:{}", change.path)
    };
    Ok(git_show_content(cwd, &spec)?.unwrap_or_default())
}

fn normalize_content_before_render(
    baseline_content: &str,
    current_content: &str,
    config: &AppConfig,
    path: &str,
) -> Result<NormalizationResult> {
    let mut current_lines = current_content
        .lines()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    let block_patterns = build_block_patterns(config)?;
    let mut blocks = find_candidate_blocks(&current_lines, config, &block_patterns, path)?;

    let added_lines = added_line_numbers(baseline_content, current_content)?;
    for block in &mut blocks {
        let is_pending = block
            .shell_lines
            .iter()
            .all(|line_idx| added_lines.contains(&(line_idx + 1)));
        if !is_pending {
            block.shell_lines.clear();
        }
    }

    let pending = blocks
        .into_iter()
        .filter(|block| !block.shell_lines.is_empty())
        .collect::<Vec<_>>();
    let mut context_candidates = pending
        .iter()
        .map(|block| block.context_candidate.clone())
        .collect::<Vec<_>>();
    context_candidates.retain(|candidate| {
        candidate.reason.is_some()
            || candidate.reference_kind.is_some()
            || candidate.reference_value.is_some()
    });

    if !pending.is_empty() {
        let normalized_lines = restore_pending_blocks(&current_lines, &pending, path)?;
        current_lines = normalized_lines;
    }

    let logical_content = if current_lines.is_empty() {
        String::new()
    } else {
        let mut content = current_lines.join("\n");
        if current_content.ends_with('\n') {
            content.push('\n');
        }
        content
    };
    let segments = diff_segments_between_contents(baseline_content, &logical_content)?;
    Ok(NormalizationResult {
        logical_content,
        segments,
        context_candidates,
    })
}

fn build_block_patterns(config: &AppConfig) -> Result<Vec<BlockPattern>> {
    let mut patterns = Vec::new();
    for (kind, template) in [
        (ChangeKind::Add, &config.annotate.block_templates.add),
        (ChangeKind::Modify, &config.annotate.block_templates.modify),
        (ChangeKind::Delete, &config.annotate.block_templates.del),
    ] {
        let start_lines = template
            .start
            .lines()
            .map(build_pattern_line)
            .collect::<Result<Vec<_>>>()?;
        let end_lines = template
            .end
            .lines()
            .map(build_pattern_line)
            .collect::<Result<Vec<_>>>()?;
        if start_lines.is_empty() || end_lines.is_empty() {
            continue;
        }
        patterns.push(BlockPattern {
            kind,
            start_contains_old_placeholder: template.start.contains("{old}"),
            start_lines,
            end_lines,
        });
    }
    Ok(patterns)
}

fn build_pattern_line(template_line: &str) -> Result<PatternLine> {
    Ok(PatternLine {
        matcher: Regex::new(&template_line_to_match_regex(template_line))
            .with_context(|| format!("invalid matcher regex for template line: {template_line}"))?,
        capture: Regex::new(&template_line_to_capture_regex(template_line))
            .with_context(|| format!("invalid capture regex for template line: {template_line}"))?,
    })
}

#[derive(Debug, Clone)]
enum TemplateToken {
    Literal(String),
    Placeholder(String),
}

fn tokenize_template_line(template_line: &str) -> Vec<TemplateToken> {
    let mut tokens = Vec::new();
    let mut cursor = 0usize;
    while cursor < template_line.len() {
        let remaining = &template_line[cursor..];
        let Some(open_idx) = remaining.find('{') else {
            tokens.push(TemplateToken::Literal(remaining.to_string()));
            break;
        };

        if open_idx > 0 {
            tokens.push(TemplateToken::Literal(remaining[..open_idx].to_string()));
        }
        let start = cursor + open_idx;
        let Some(close_rel) = template_line[start + 1..].find('}') else {
            tokens.push(TemplateToken::Literal(template_line[start..].to_string()));
            break;
        };
        let close = start + 1 + close_rel;
        let name = &template_line[start + 1..close];
        if !name.is_empty() && name.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_') {
            tokens.push(TemplateToken::Placeholder(name.to_string()));
        } else {
            tokens.push(TemplateToken::Literal(
                template_line[start..=close].to_string(),
            ));
        }
        cursor = close + 1;
    }
    tokens
}

fn has_non_empty_literal_after(tokens: &[TemplateToken], index: usize) -> bool {
    tokens.iter().skip(index + 1).any(|token| match token {
        TemplateToken::Literal(text) => !text.is_empty(),
        TemplateToken::Placeholder(_) => false,
    })
}

fn template_line_to_match_regex(template_line: &str) -> String {
    let mut pattern = String::from("^");
    for token in tokenize_template_line(template_line) {
        match token {
            TemplateToken::Literal(text) => pattern.push_str(&regex::escape(&text)),
            TemplateToken::Placeholder(_) => pattern.push_str(".*"),
        }
    }
    pattern.push('$');
    pattern
}

fn template_line_to_capture_regex(template_line: &str) -> String {
    let tokens = tokenize_template_line(template_line);
    let mut pattern = String::from("^");
    for (idx, token) in tokens.iter().enumerate() {
        match token {
            TemplateToken::Literal(text) => pattern.push_str(&regex::escape(text)),
            TemplateToken::Placeholder(name) => {
                let wildcard = if has_non_empty_literal_after(&tokens, idx) {
                    ".*?"
                } else {
                    ".*"
                };
                match name.as_str() {
                    "reason" | "reference_kind" | "reference_value" => {
                        pattern.push_str(&format!("(?P<{name}>{wildcard})"))
                    }
                    _ => pattern.push_str(wildcard),
                }
            }
        }
    }
    pattern.push('$');
    pattern
}

fn find_candidate_blocks(
    lines: &[String],
    config: &AppConfig,
    block_patterns: &[BlockPattern],
    path: &str,
) -> Result<Vec<CandidateAnnotationBlock>> {
    let mut blocks = Vec::new();
    for line_idx in 0..lines.len() {
        let mut matches = Vec::new();
        for pattern in block_patterns {
            if let Some((indent, context_candidate)) =
                try_match_block_start(lines, line_idx, pattern)
            {
                matches.push((pattern, indent, context_candidate));
            }
        }
        if matches.is_empty() {
            continue;
        }
        matches.sort_by_key(|(pattern, _, _)| std::cmp::Reverse(pattern.start_lines.len()));
        let (pattern, indent, mut context_candidate) = matches.remove(0);
        normalize_context_candidate(&mut context_candidate, &config.annotate.reference_kinds);
        let start_len = pattern.start_lines.len();
        let search_start = line_idx + start_len;
        let Some(end_start) = find_matching_end(lines, search_start, pattern, &indent) else {
            bail!(
                "{}: unmatched annotation block start at line {}",
                path,
                line_idx + 1
            );
        };
        let end_len = pattern.end_lines.len();
        let end_line = end_start + end_len;
        let interior = &lines[search_start..end_start];
        let old_region_len = parse_old_region_len(
            &pattern.kind,
            pattern.start_contains_old_placeholder,
            interior,
            config,
            &indent,
            path,
            line_idx + 1,
        )?;
        if old_region_len > interior.len() {
            bail!("{}: failed to determine old-code region", path);
        }
        let code_start_line = search_start + old_region_len;
        let code_end_line = end_start;
        if pattern.kind == ChangeKind::Delete {
            if code_start_line != code_end_line {
                bail!("{}: delete block contains ambiguous code body", path);
            }
        } else if code_start_line >= code_end_line {
            bail!(
                "{}: block at line {} has empty code body",
                path,
                line_idx + 1
            );
        }

        let mut shell_lines = Vec::new();
        shell_lines.extend(line_idx..search_start);
        shell_lines.extend(search_start..code_start_line);
        shell_lines.extend(end_start..end_line);
        blocks.push(CandidateAnnotationBlock {
            kind: pattern.kind.clone(),
            start_line: line_idx,
            end_line,
            code_start_line,
            code_end_line,
            shell_lines,
            context_candidate,
        });
    }
    Ok(blocks)
}

fn try_match_block_start(
    lines: &[String],
    line_idx: usize,
    pattern: &BlockPattern,
) -> Option<(String, ContextReuseCandidate)> {
    if line_idx + pattern.start_lines.len() > lines.len() {
        return None;
    }
    let first_line = &lines[line_idx];
    let indent = first_line
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .collect::<String>();
    let mut candidate = ContextReuseCandidate::default();
    for (offset, pattern_line) in pattern.start_lines.iter().enumerate() {
        let line = lines.get(line_idx + offset)?;
        let content = line.strip_prefix(&indent)?;
        if !pattern_line.matcher.is_match(content) {
            return None;
        }
        if let Some(caps) = pattern_line.capture.captures(content) {
            for (field, slot) in [
                ("reason", &mut candidate.reason),
                ("reference_kind", &mut candidate.reference_kind),
                ("reference_value", &mut candidate.reference_value),
            ] {
                if let Some(value) = caps.name(field) {
                    let value = value.as_str().trim().to_string();
                    if value.is_empty() {
                        continue;
                    }
                    if let Some(existing) = slot {
                        if existing != &value {
                            *slot = None;
                        }
                    } else {
                        *slot = Some(value);
                    }
                }
            }
        }
    }
    Some((indent, candidate))
}

fn match_pattern_with_indent(
    lines: &[String],
    start: usize,
    pattern_lines: &[PatternLine],
    indent: &str,
) -> bool {
    if start + pattern_lines.len() > lines.len() {
        return false;
    }
    pattern_lines
        .iter()
        .enumerate()
        .all(|(offset, pattern_line)| {
            lines
                .get(start + offset)
                .and_then(|line| line.strip_prefix(indent))
                .is_some_and(|content| pattern_line.matcher.is_match(content))
        })
}

fn find_matching_end(
    lines: &[String],
    mut cursor: usize,
    pattern: &BlockPattern,
    indent: &str,
) -> Option<usize> {
    let mut nesting = 0usize;
    while cursor < lines.len() {
        if match_pattern_with_indent(lines, cursor, &pattern.start_lines, indent) {
            nesting += 1;
            cursor += pattern.start_lines.len();
            continue;
        }
        if match_pattern_with_indent(lines, cursor, &pattern.end_lines, indent) {
            if nesting == 0 {
                return Some(cursor);
            }
            nesting = nesting.saturating_sub(1);
            cursor += pattern.end_lines.len();
            continue;
        }
        cursor += 1;
    }
    None
}

fn parse_old_region_len(
    kind: &ChangeKind,
    start_contains_old_placeholder: bool,
    interior: &[String],
    config: &AppConfig,
    indent: &str,
    path: &str,
    start_line: usize,
) -> Result<usize> {
    if *kind == ChangeKind::Add {
        return Ok(0);
    }
    match &config.annotate.old_code.mode {
        Some(AnnotateOldCodeMode::None) => Ok(0),
        Some(AnnotateOldCodeMode::LineComment) => {
            parse_line_comment_old_region(interior, &config.annotate.old_code.line_comment, indent)
                .with_context(|| {
                    format!("{path}: failed to parse line-comment old region at line {start_line}")
                })
        }
        Some(AnnotateOldCodeMode::BlockComment) => parse_block_comment_old_region(
            interior,
            &config.annotate.old_code.block_comment,
            indent,
        )
        .with_context(|| {
            format!("{path}: failed to parse block-comment old region at line {start_line}")
        }),
        None => {
            if start_contains_old_placeholder {
                return Ok(0);
            }
            if interior.is_empty() {
                bail!("{path}: legacy old-code section missing at line {start_line}");
            }
            let header = c_line_comment("old:", indent);
            if interior[0] != header {
                bail!("{path}: legacy old-code header missing at line {start_line}");
            }
            let mut consumed = 1usize;
            let old_prefix = format!("{indent}//   ");
            while consumed < interior.len() {
                if interior[consumed].starts_with(&old_prefix) {
                    consumed += 1;
                } else {
                    break;
                }
            }
            Ok(consumed)
        }
    }
}

fn parse_line_comment_old_region(
    interior: &[String],
    config: &AnnotateOldCodeLineCommentConfig,
    indent: &str,
) -> Result<usize> {
    if interior.is_empty() {
        bail!("missing old-code section");
    }
    let mut index = 0usize;
    if config.layout == AnnotateOldCodeLineLayout::HeaderBody && !config.header.trim().is_empty() {
        let expected = c_line_comment(config.header.as_str(), indent);
        if interior.get(index) != Some(&expected) {
            bail!("line-comment header not found");
        }
        index += 1;
    }

    let line_prefix = format!("{indent}// ");
    let mut consumed_body = 0usize;
    while let Some(line) = interior.get(index) {
        let Some(payload) = line.strip_prefix(&line_prefix) else {
            break;
        };
        if !payload.starts_with(&config.body_prefix) {
            break;
        }
        if !config.body_suffix.is_empty() && !payload.ends_with(&config.body_suffix) {
            break;
        }
        consumed_body += 1;
        index += 1;
    }
    if consumed_body == 0 {
        bail!("line-comment old-code body not found");
    }
    Ok(index)
}

fn parse_block_comment_old_region(
    interior: &[String],
    config: &AnnotateOldCodeBlockCommentConfig,
    indent: &str,
) -> Result<usize> {
    if interior.is_empty() {
        bail!("missing block-comment old-code section");
    }
    let header = if config.title.trim().is_empty() {
        format!("{indent}/*")
    } else {
        format!("{indent}/* {}", config.title)
    };
    if interior[0] != header {
        bail!("block-comment header not found");
    }

    let body_prefix = format!("{indent} * {}", config.body_prefix);
    let end_line = format!("{indent} */");
    let mut index = 1usize;
    let mut seen_body = false;
    while let Some(line) = interior.get(index) {
        if line == &end_line {
            if !seen_body {
                bail!("block-comment old-code body not found");
            }
            return Ok(index + 1);
        }
        if line.starts_with(&body_prefix) {
            seen_body = true;
            index += 1;
            continue;
        }
        bail!("invalid block-comment old-code line");
    }
    bail!("block-comment old-code terminator not found")
}

fn added_line_numbers(baseline_content: &str, current_content: &str) -> Result<BTreeSet<usize>> {
    let mut added = BTreeSet::new();
    for segment in diff_segments_between_contents(baseline_content, current_content)? {
        if segment.kind == ChangeKind::Delete {
            continue;
        }
        for offset in 0..segment.new_lines.len() {
            added.insert(segment.start_line + offset);
        }
    }
    Ok(added)
}

fn restore_pending_blocks(
    original_lines: &[String],
    pending_blocks: &[CandidateAnnotationBlock],
    path: &str,
) -> Result<Vec<String>> {
    let mut blocks = pending_blocks.to_vec();
    blocks.sort_by_key(|block| (block.start_line, std::cmp::Reverse(block.end_line)));

    let mut children = vec![Vec::<usize>::new(); blocks.len()];
    let mut roots = Vec::<usize>::new();
    let mut stack = Vec::<usize>::new();
    for index in 0..blocks.len() {
        while let Some(&last) = stack.last() {
            if blocks[index].start_line >= blocks[last].end_line {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some(&parent) = stack.last() {
            if blocks[index].end_line > blocks[parent].end_line {
                bail!("{path}: overlapping pending annotation blocks");
            }
            if blocks[index].start_line < blocks[parent].code_start_line
                || blocks[index].end_line > blocks[parent].code_end_line
            {
                bail!("{path}: pending block overlaps annotation shell");
            }
            children[parent].push(index);
        } else {
            roots.push(index);
        }
        stack.push(index);
    }

    Ok(render_range_without_shell(
        0,
        original_lines.len(),
        &roots,
        original_lines,
        &blocks,
        &children,
    ))
}

fn render_range_without_shell(
    start: usize,
    end: usize,
    block_indices: &[usize],
    original_lines: &[String],
    blocks: &[CandidateAnnotationBlock],
    children: &[Vec<usize>],
) -> Vec<String> {
    let mut output = Vec::new();
    let mut cursor = start;
    for &index in block_indices {
        let block = &blocks[index];
        while cursor < block.start_line {
            output.push(original_lines[cursor].clone());
            cursor += 1;
        }
        output.extend(render_block_without_shell(
            index,
            original_lines,
            blocks,
            children,
        ));
        cursor = block.end_line;
    }
    while cursor < end {
        output.push(original_lines[cursor].clone());
        cursor += 1;
    }
    output
}

fn render_block_without_shell(
    index: usize,
    original_lines: &[String],
    blocks: &[CandidateAnnotationBlock],
    children: &[Vec<usize>],
) -> Vec<String> {
    let block = &blocks[index];
    if block.kind == ChangeKind::Delete {
        return Vec::new();
    }
    render_range_without_shell(
        block.code_start_line,
        block.code_end_line,
        &children[index],
        original_lines,
        blocks,
        children,
    )
}

fn diff_segments_between_contents(
    baseline_content: &str,
    current_content: &str,
) -> Result<Vec<HunkSegment>> {
    let patch = diff_patch_between_contents(baseline_content, current_content)?;
    Ok(parse_hunk_segments(&patch))
}

fn diff_patch_between_contents(baseline_content: &str, current_content: &str) -> Result<String> {
    let temp_base = create_temp_file_with_content("xgit-annotate-base", baseline_content)?;
    let temp_current = create_temp_file_with_content("xgit-annotate-current", current_content)?;
    let output = Command::new("git")
        .args([
            "diff",
            "--no-index",
            "--unified=0",
            "--",
            temp_base.to_string_lossy().as_ref(),
            temp_current.to_string_lossy().as_ref(),
        ])
        .output()
        .context("failed to run git diff --no-index")?;
    let _ = fs::remove_file(&temp_base);
    let _ = fs::remove_file(&temp_current);

    match output.status.code() {
        Some(0) | Some(1) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            bail!("git diff --no-index failed: {stderr}");
        }
    }
}

fn create_temp_file_with_content(prefix: &str, content: &str) -> Result<PathBuf> {
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let filename = format!("{prefix}-{nanos}-{counter}.tmp");
    let path = std::env::temp_dir().join(filename);
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
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
    let filtered = changes
        .into_iter()
        .filter(|change| !is_repo_xgit_path(&change.path))
        .collect::<Vec<_>>();
    dedup_changes(filtered)
}

fn collect_latest_commit_changes(cwd: &Path, catalog: &Catalog) -> Result<Vec<FileChange>> {
    validate_latest_commit_mode(cwd, catalog)?;
    let output = git_stdout(cwd, &["diff", "--name-status", "HEAD^", "HEAD"])?;
    let changes = parse_name_status_output(&output)
        .into_iter()
        .filter(|change| !is_repo_xgit_path(&change.path))
        .collect::<Vec<_>>();
    dedup_changes(changes)
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
    let has_non_xgit_dirty = status
        .lines()
        .filter(|line| !line.trim().is_empty())
        .any(|line| !is_ignorable_status_line(line));
    if has_non_xgit_dirty {
        bail!("{}", catalog.t("error.annotate.latest_commit_dirty"));
    }

    Ok(())
}

fn is_repo_xgit_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized == ".xgit" || normalized.starts_with(".xgit/")
}

fn is_ignorable_status_line(line: &str) -> bool {
    parse_status_paths(line)
        .iter()
        .all(|path| is_repo_xgit_path(path))
}

fn parse_status_paths(line: &str) -> Vec<String> {
    if line.len() < 4 {
        return Vec::new();
    }
    let payload = line[3..].trim();
    if payload.is_empty() {
        return Vec::new();
    }
    if let Some((from, to)) = payload.split_once(" -> ") {
        return vec![normalize_status_path(from), normalize_status_path(to)];
    }
    vec![normalize_status_path(payload)]
}

fn normalize_status_path(path: &str) -> String {
    path.trim().trim_matches('"').to_string()
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
        let Some(window) = segment_render_window(segment, config) else {
            continue;
        };

        let (prefix, suffix) =
            render_c_line_block(segment, context, config, path, window.code_lines_range);
        let base_index = segment.start_line.saturating_sub(1).min(lines.len());
        let insert_index = (base_index + window.insert_offset).min(lines.len());
        lines.splice(insert_index..insert_index, prefix.clone());

        let suffix_index = if segment.kind == ChangeKind::Delete {
            insert_index + prefix.len()
        } else {
            (insert_index + prefix.len() + window.covered_line_count).min(lines.len())
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
    code_lines_range: Option<(usize, usize)>,
) -> (Vec<String>, Vec<String>) {
    let explicit_old_code_mode = config.annotate.old_code.mode.clone();
    let old_value_override = explicit_old_code_mode.as_ref().map(|_| "");

    let (start_template, end_template) = match segment.kind {
        ChangeKind::Add => (
            &config.annotate.block_templates.add.start,
            &config.annotate.block_templates.add.end,
        ),
        ChangeKind::Modify => (
            &config.annotate.block_templates.modify.start,
            &config.annotate.block_templates.modify.end,
        ),
        ChangeKind::Delete => (
            &config.annotate.block_templates.del.start,
            &config.annotate.block_templates.del.end,
        ),
    };
    let rendered_start = expand_policy_template(
        start_template,
        context,
        segment.kind.key(),
        path,
        old_value_override,
        &segment.old_lines,
        &segment.new_lines,
    );
    let rendered_end = expand_policy_template(
        end_template,
        context,
        segment.kind.key(),
        path,
        old_value_override,
        &segment.old_lines,
        &segment.new_lines,
    );

    let comment_indent = resolve_comment_indent(segment, config, code_lines_range);
    let mut prefix = rendered_start
        .lines()
        .map(|line| format!("{}{}", comment_indent, line))
        .collect::<Vec<_>>();
    if segment.kind == ChangeKind::Modify || segment.kind == ChangeKind::Delete {
        match explicit_old_code_mode {
            Some(AnnotateOldCodeMode::None) => {}
            Some(AnnotateOldCodeMode::LineComment) => {
                prefix.extend(render_old_code_line_comment(
                    &segment.old_lines,
                    &config.annotate.old_code.line_comment,
                    comment_indent.as_str(),
                ));
            }
            Some(AnnotateOldCodeMode::BlockComment) => {
                prefix.extend(render_old_code_block_comment(
                    &segment.old_lines,
                    &config.annotate.old_code.block_comment,
                    comment_indent.as_str(),
                ));
            }
            None => {
                if !segment.old_lines.is_empty() && !start_template.contains("{old}") {
                    prefix.push(c_line_comment("old:", comment_indent.as_str()));
                    for old in &segment.old_lines {
                        prefix.push(c_line_comment(&format!("  {old}"), comment_indent.as_str()));
                    }
                }
            }
        }
    }

    let suffix = rendered_end
        .lines()
        .map(|line| format!("{}{}", comment_indent, line))
        .collect::<Vec<_>>();
    (prefix, suffix)
}

fn c_line_comment(content: &str, indent: &str) -> String {
    if content.trim().is_empty() {
        format!("{indent}//")
    } else {
        format!("{indent}// {content}")
    }
}

fn render_old_code_line_comment(
    old_lines: &[String],
    config: &AnnotateOldCodeLineCommentConfig,
    indent: &str,
) -> Vec<String> {
    if old_lines.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    if config.layout == AnnotateOldCodeLineLayout::HeaderBody && !config.header.trim().is_empty() {
        lines.push(c_line_comment(config.header.as_str(), indent));
    }
    for old_line in old_lines {
        lines.push(c_line_comment(
            format!("{}{}{}", config.body_prefix, old_line, config.body_suffix).as_str(),
            indent,
        ));
    }
    lines
}

fn render_old_code_block_comment(
    old_lines: &[String],
    config: &AnnotateOldCodeBlockCommentConfig,
    indent: &str,
) -> Vec<String> {
    if old_lines.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::with_capacity(old_lines.len() + 2);
    if config.title.trim().is_empty() {
        lines.push(format!("{indent}/*"));
    } else {
        lines.push(format!("{indent}/* {}", config.title));
    }
    for old_line in old_lines {
        if old_line.is_empty() {
            lines.push(format!("{indent} * {}", config.body_prefix));
        } else {
            lines.push(format!("{indent} * {}{}", config.body_prefix, old_line));
        }
    }
    lines.push(format!("{indent} */"));
    lines
}

#[derive(Debug, Clone, Copy)]
struct SegmentRenderWindow {
    insert_offset: usize,
    covered_line_count: usize,
    code_lines_range: Option<(usize, usize)>,
}

fn segment_render_window(segment: &HunkSegment, config: &AppConfig) -> Option<SegmentRenderWindow> {
    if segment.kind == ChangeKind::Delete {
        return Some(SegmentRenderWindow {
            insert_offset: 0,
            covered_line_count: 0,
            code_lines_range: None,
        });
    }

    if config.annotate.render.wrap_blank_lines {
        return Some(SegmentRenderWindow {
            insert_offset: 0,
            covered_line_count: segment.new_lines.len(),
            code_lines_range: Some((0, segment.new_lines.len())),
        });
    }

    let first = segment
        .new_lines
        .iter()
        .position(|line| !line.trim().is_empty())?;
    let last = segment
        .new_lines
        .iter()
        .rposition(|line| !line.trim().is_empty())?;
    let len = last - first + 1;
    Some(SegmentRenderWindow {
        insert_offset: first,
        covered_line_count: len,
        code_lines_range: Some((first, len)),
    })
}

fn resolve_comment_indent(
    segment: &HunkSegment,
    config: &AppConfig,
    code_lines_range: Option<(usize, usize)>,
) -> String {
    if !config.annotate.render.align_with_code_indent {
        return String::new();
    }

    let candidate_lines: &[String] =
        if segment.kind == ChangeKind::Delete || segment.new_lines.is_empty() {
            &segment.old_lines
        } else if let Some((start, len)) = code_lines_range {
            &segment.new_lines[start..start + len]
        } else {
            &segment.new_lines
        };

    candidate_lines
        .iter()
        .find_map(|line| {
            if line.trim().is_empty() {
                None
            } else {
                Some(
                    line.chars()
                        .take_while(|ch| ch.is_whitespace())
                        .collect::<String>(),
                )
            }
        })
        .unwrap_or_default()
}

fn expand_policy_template(
    template: &str,
    context: &RuntimeContext,
    kind: &str,
    path: &str,
    old_value_override: Option<&str>,
    old_lines: &[String],
    new_lines: &[String],
) -> String {
    let old = old_value_override
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| old_lines.join("\\n"));
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
        apply_c_line_segments, build_annotate_report_lines, classify_missing_rule_reason,
        collect_latest_commit_changes, collect_runtime_context, collect_staged_changes, git_stdout,
        load_baseline_content, matches_pattern, normalize_content_before_render,
        parse_hunk_segments, parse_name_status_output, resolve_reusable_context_defaults, run,
        select_renderer, ChangeKind, ContextReuseCandidate, FileChange, HunkSegment, RuntimeContext,
        UnformattedFile, UnformattedReason,
    };
    use crate::code_file_types::{default_selected_keys, file_rules_from_selection};
    use crate::config::{
        load_runtime_config, merge_layers, AnnotateOldCodeLineLayout, AnnotateOldCodeMode, AppConfig,
        LoadConfigOptions,
    };
    use chrono::NaiveDateTime;
    use std::collections::HashMap;
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
    fn expanded_code_file_rules_match_code_files_but_not_android_bp() {
        let mut selected = default_selected_keys();
        selected.insert("javascript/mjs".to_string());
        selected.insert("kotlin/kts".to_string());

        let rules = file_rules_from_selection(&selected, &[]);

        assert_eq!(
            select_renderer(
                "networkmgr/routemgr/include/proxy_messsage_handler.hpp",
                &rules
            ),
            Some("c_line_block".to_string())
        );
        assert_eq!(
            select_renderer("web/src/app.mjs", &rules),
            Some("c_line_block".to_string())
        );
        assert_eq!(
            select_renderer("gradle/build.kts", &rules),
            Some("c_line_block".to_string())
        );
        assert_eq!(select_renderer("build/Android.bp", &rules), None);
    }

    #[test]
    fn classify_missing_rule_marks_unknown_type_as_unsupported() {
        assert_eq!(
            classify_missing_rule_reason("build/Android.bp"),
            UnformattedReason::UnsupportedType
        );
    }

    #[test]
    fn classify_missing_rule_marks_builtin_suffix_as_disabled() {
        assert_eq!(
            classify_missing_rule_reason("networkmgr/routemgr/DnsEvent.cpp"),
            UnformattedReason::BuiltinTypeDisabled
        );
    }

    #[test]
    fn annotate_report_lists_reasons_for_unformatted_files() {
        let catalog = crate::i18n::load_catalog("zh-CN", Path::new(".")).unwrap();
        let lines = build_annotate_report_lines(
            &catalog,
            1,
            &[
                UnformattedFile {
                    path: "build/Android.bp".to_string(),
                    reason: UnformattedReason::UnsupportedType,
                },
                UnformattedFile {
                    path: "src/demo.cpp".to_string(),
                    reason: UnformattedReason::BuiltinTypeDisabled,
                },
                UnformattedFile {
                    path: "src/custom.proto".to_string(),
                    reason: UnformattedReason::RendererUnimplemented(
                        "proto_line_block".to_string(),
                    ),
                },
            ],
        );
        let output = lines.join("\n");
        assert!(output.contains("未格式化：build/Android.bp (不支持该类型)"));
        assert!(output.contains("未格式化：src/demo.cpp (设置未开启该后缀功能)"));
        assert!(output.contains("命中了尚未实现的渲染器 'proto_line_block'"));
    }

    #[test]
    fn annotate_report_hides_unformatted_list_when_count_is_zero() {
        let catalog = crate::i18n::load_catalog("zh-CN", Path::new(".")).unwrap();
        let lines = build_annotate_report_lines(&catalog, 2, &[]);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("已渲染 2，未格式化 0"));
    }

    #[test]
    fn apply_segments_wraps_changed_lines() {
        let mut cfg = AppConfig::default();
        cfg.annotate.block_templates.add.start =
            "// add {author_tag}:{reason}:{reference_kind}:{reference_value}".to_string();
        cfg.annotate.block_templates.add.end = "// end add".to_string();
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
    fn apply_segments_honors_custom_end_template() {
        let mut cfg = AppConfig::default();
        cfg.annotate.block_templates.add.start = "// add block {@".to_string();
        cfg.annotate.block_templates.add.end = "//@}".to_string();
        let content = "int a = 1;\n";
        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 1,
                kind: ChangeKind::Add,
                old_lines: vec![],
                new_lines: vec!["int a = 1;".to_string()],
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

        assert!(updated.contains("// add block {@"));
        assert!(updated.contains("//@}"));
        assert!(!updated.contains("// end add"));
    }

    #[test]
    fn apply_segments_renders_with_legacy_policy_fallback_config() {
        let project = r#"
[annotate.policies]
add = "legacy {author_tag}:{reason}"
"#;
        let cfg = merge_layers(None, Some(project), &HashMap::new(), "zh-CN").unwrap();
        let content = "int a = 1;\n";
        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 1,
                kind: ChangeKind::Add,
                old_lines: vec![],
                new_lines: vec!["int a = 1;".to_string()],
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

        assert!(updated.contains("// legacy QA:why"));
        assert!(updated.contains("// end add"));
    }

    #[test]
    fn apply_segments_renders_with_block_templates_from_config_layer() {
        let project = r#"
[annotate.block_templates.add]
start = "// add by {author_tag} {@"
end = "//@}"
"#;
        let cfg = merge_layers(None, Some(project), &HashMap::new(), "zh-CN").unwrap();
        let content = "int a = 1;\n";
        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 1,
                kind: ChangeKind::Add,
                old_lines: vec![],
                new_lines: vec!["int a = 1;".to_string()],
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

        assert!(updated.contains("// add by QA {@"));
        assert!(updated.contains("//@}"));
    }

    #[test]
    fn apply_segments_aligns_with_code_indent_when_enabled() {
        let mut cfg = AppConfig::default();
        cfg.annotate.render.align_with_code_indent = true;
        cfg.annotate.block_templates.add.start = "// aligned".to_string();
        cfg.annotate.block_templates.add.end = "// end add".to_string();
        let content = "    int value = 42;\n";

        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 1,
                kind: ChangeKind::Add,
                old_lines: vec![],
                new_lines: vec!["    int value = 42;".to_string()],
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

        assert!(updated.contains("    // aligned"));
        assert!(updated.contains("    // end add"));
    }

    #[test]
    fn apply_segments_wrap_blank_lines_option_controls_comment_boundary() {
        let mut cfg = AppConfig::default();
        cfg.annotate.block_templates.add.start = "// boundary".to_string();
        cfg.annotate.block_templates.add.end = "// end add".to_string();
        let segment = HunkSegment {
            start_line: 1,
            kind: ChangeKind::Add,
            old_lines: vec![],
            new_lines: vec![
                "".to_string(),
                "    int value = 42;".to_string(),
                "".to_string(),
            ],
        };
        let context = RuntimeContext {
            reason: "why".to_string(),
            reference_kind: "bug".to_string(),
            reference_value: "ID-1".to_string(),
            author_tag: "QA".to_string(),
            date: "123".to_string(),
        };
        let content = "\n    int value = 42;\n\n";

        cfg.annotate.render.wrap_blank_lines = true;
        let wrapped = apply_c_line_segments(content, &[segment.clone()], &context, &cfg, "demo.c");
        assert!(wrapped
            .lines()
            .next()
            .expect("at least one output line")
            .starts_with("//"));

        cfg.annotate.render.wrap_blank_lines = false;
        let trimmed = apply_c_line_segments(content, &[segment], &context, &cfg, "demo.c");
        assert_eq!(trimmed.lines().next().unwrap_or_default(), "");
        assert!(trimmed
            .lines()
            .nth(1)
            .expect("comment line after leading blank")
            .starts_with("//"));
    }

    #[test]
    fn date_format_default_pattern_outputs_expected_date_text() {
        let date = NaiveDateTime::parse_from_str("2026-04-01 09:08:07", "%Y-%m-%d %H:%M:%S")
            .expect("test datetime");
        assert_eq!(super::format_date_tag("yyyy-mm-dd", date), "2026-04-01");
    }

    #[test]
    fn date_format_custom_pattern_outputs_expected_date_text() {
        let date = NaiveDateTime::parse_from_str("2026-04-01 09:08:07", "%Y-%m-%d %H:%M:%S")
            .expect("test datetime");
        assert_eq!(
            super::format_date_tag("dd/mm/yyyy HH:MM", date),
            "01/04/2026 09:08"
        );
    }

    #[test]
    fn explicit_old_code_none_suppresses_old_content_rendering() {
        let mut cfg = AppConfig::default();
        cfg.annotate.old_code.mode = Some(AnnotateOldCodeMode::None);

        let content = "int a = 2;\n";
        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 1,
                kind: ChangeKind::Modify,
                old_lines: vec!["int a = 1;".to_string()],
                new_lines: vec!["int a = 2;".to_string()],
            }],
            &RuntimeContext {
                reason: "why".to_string(),
                reference_kind: "bug".to_string(),
                reference_value: "ID-1".to_string(),
                author_tag: "QA".to_string(),
                date: "2026-04-01".to_string(),
            },
            &cfg,
            "demo.c",
        );

        assert!(!updated.contains("int a = 1;"));
        assert!(!updated.contains("// old:"));
    }

    #[test]
    fn explicit_old_code_line_comment_per_line_renders_each_old_line() {
        let mut cfg = AppConfig::default();
        cfg.annotate.old_code.mode = Some(AnnotateOldCodeMode::LineComment);
        cfg.annotate.old_code.line_comment.layout = AnnotateOldCodeLineLayout::PerLine;
        cfg.annotate.old_code.line_comment.body_prefix = "old: ".to_string();
        cfg.annotate.old_code.line_comment.body_suffix = "".to_string();
        cfg.annotate.block_templates.modify.start = "// modify".to_string();
        cfg.annotate.block_templates.modify.end = "// end modify".to_string();

        let content = "int a = 2;\nint b = 3;\n";
        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 1,
                kind: ChangeKind::Modify,
                old_lines: vec!["int a = 1;".to_string(), "int b = 2;".to_string()],
                new_lines: vec!["int a = 2;".to_string(), "int b = 3;".to_string()],
            }],
            &RuntimeContext {
                reason: "why".to_string(),
                reference_kind: "bug".to_string(),
                reference_value: "ID-1".to_string(),
                author_tag: "QA".to_string(),
                date: "2026-04-01".to_string(),
            },
            &cfg,
            "demo.c",
        );

        assert!(updated.contains("// old: int a = 1;"));
        assert!(updated.contains("// old: int b = 2;"));
    }

    #[test]
    fn explicit_old_code_line_comment_header_body_renders_header_and_body() {
        let mut cfg = AppConfig::default();
        cfg.annotate.old_code.mode = Some(AnnotateOldCodeMode::LineComment);
        cfg.annotate.old_code.line_comment.layout = AnnotateOldCodeLineLayout::HeaderBody;
        cfg.annotate.old_code.line_comment.header = "legacy old".to_string();
        cfg.annotate.old_code.line_comment.body_prefix = ">> ".to_string();
        cfg.annotate.old_code.line_comment.body_suffix = String::new();
        cfg.annotate.block_templates.modify.start = "// modify".to_string();
        cfg.annotate.block_templates.modify.end = "// end modify".to_string();

        let content = "int a = 2;\n";
        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 1,
                kind: ChangeKind::Modify,
                old_lines: vec!["int a = 1;".to_string()],
                new_lines: vec!["int a = 2;".to_string()],
            }],
            &RuntimeContext {
                reason: "why".to_string(),
                reference_kind: "bug".to_string(),
                reference_value: "ID-1".to_string(),
                author_tag: "QA".to_string(),
                date: "2026-04-01".to_string(),
            },
            &cfg,
            "demo.c",
        );

        assert!(updated.contains("// legacy old"));
        assert!(updated.contains("// >> int a = 1;"));
    }

    #[test]
    fn explicit_old_code_block_comment_renders_block() {
        let mut cfg = AppConfig::default();
        cfg.annotate.old_code.mode = Some(AnnotateOldCodeMode::BlockComment);
        cfg.annotate.old_code.block_comment.title = "cover old codes".to_string();
        cfg.annotate.old_code.block_comment.body_prefix = "| ".to_string();
        cfg.annotate.block_templates.modify.start = "// modify".to_string();
        cfg.annotate.block_templates.modify.end = "// end modify".to_string();

        let content = "int a = 2;\n";
        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 1,
                kind: ChangeKind::Modify,
                old_lines: vec!["int a = 1;".to_string()],
                new_lines: vec!["int a = 2;".to_string()],
            }],
            &RuntimeContext {
                reason: "why".to_string(),
                reference_kind: "bug".to_string(),
                reference_value: "ID-1".to_string(),
                author_tag: "QA".to_string(),
                date: "2026-04-01".to_string(),
            },
            &cfg,
            "demo.c",
        );

        assert!(updated.contains("/* cover old codes"));
        assert!(updated.contains(" * | int a = 1;"));
        assert!(updated.contains(" */"));
    }

    #[test]
    fn legacy_old_code_fallback_kept_when_mode_is_unset() {
        let mut cfg = AppConfig::default();
        cfg.annotate.old_code.mode = None;
        cfg.annotate.block_templates.modify.start = "// modify".to_string();
        cfg.annotate.block_templates.modify.end = "// end modify".to_string();

        let content = "int a = 2;\n";
        let updated = apply_c_line_segments(
            content,
            &[HunkSegment {
                start_line: 1,
                kind: ChangeKind::Modify,
                old_lines: vec!["int a = 1;".to_string()],
                new_lines: vec!["int a = 2;".to_string()],
            }],
            &RuntimeContext {
                reason: "why".to_string(),
                reference_kind: "bug".to_string(),
                reference_value: "ID-1".to_string(),
                author_tag: "QA".to_string(),
                date: "2026-04-01".to_string(),
            },
            &cfg,
            "demo.c",
        );

        assert!(updated.contains("// old:"));
        assert!(updated.contains("//   int a = 1;"));
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
            None,
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
    fn staged_source_ignores_repo_xgit_paths() {
        let repo = init_test_repo();
        fs::write(repo.path().join(".xgit").join("cache.txt"), "tmp\n").unwrap();
        fs::write(repo.path().join("tracked.c"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "tracked.c"]);

        let changes = collect_staged_changes(repo.path(), true).unwrap();
        assert!(changes.iter().any(|c| c.path == "tracked.c"));
        assert!(!changes.iter().any(|c| c.path.starts_with(".xgit")));
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
    fn latest_commit_allows_dirty_repo_xgit_only() {
        let repo = init_test_repo();
        fs::write(repo.path().join("demo.c"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "init"]);

        fs::write(repo.path().join("demo.c"), "int a = 2;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "update"]);

        fs::write(repo.path().join(".xgit").join("runtime.state"), "dirty\n").unwrap();
        let catalog = crate::i18n::load_catalog("en-US", repo.path()).unwrap();
        let changes = collect_latest_commit_changes(repo.path(), &catalog).unwrap();
        assert!(changes.iter().any(|c| c.path == "demo.c"));
    }

    #[test]
    fn latest_commit_rejects_other_dirty_paths() {
        let repo = init_test_repo();
        fs::write(repo.path().join("demo.c"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "init"]);

        fs::write(repo.path().join("demo.c"), "int a = 2;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "update"]);

        fs::write(repo.path().join("other.txt"), "dirty\n").unwrap();
        let catalog = crate::i18n::load_catalog("en-US", repo.path()).unwrap();
        let err = collect_latest_commit_changes(repo.path(), &catalog).unwrap_err();
        assert!(err.to_string().contains("clean working tree"));
    }

    #[test]
    fn annotate_with_repo_config_still_works_when_repo_xgit_is_dirty() {
        let repo = init_test_repo();
        fs::write(repo.path().join("demo.c"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "init"]);

        fs::write(repo.path().join("demo.c"), "int a = 2;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "update"]);

        let project_cfg = r#"
[annotate.block_templates.modify]
start = "// repo-template {reason}"
end = "// repo-end"
"#;
        fs::write(repo.path().join(".xgit").join("config.toml"), project_cfg).unwrap();
        fs::write(repo.path().join(".xgit").join("runtime.state"), "dirty\n").unwrap();

        let runtime = load_runtime_config(repo.path(), &LoadConfigOptions).unwrap();
        let catalog = crate::i18n::load_catalog("en-US", repo.path()).unwrap();
        run(
            super::AnnotateOptions {
                latest_commit: true,
                include_untracked_override: None,
                reason: Some("from-repo-config".to_string()),
                reference_kind: Some("bug".to_string()),
                reference_value: Some("ID-9".to_string()),
            },
            &runtime.effective,
            &catalog,
            repo.path(),
        )
        .unwrap();

        let file = fs::read_to_string(repo.path().join("demo.c")).unwrap();
        assert!(file.contains("// repo-template from-repo-config"));
        assert!(file.contains("// repo-end"));
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

    #[test]
    fn pending_add_blocks_rebuild_to_single_add_wrapper() {
        let mut cfg = AppConfig::default();
        cfg.annotate.block_templates.add.start = "// add {reason}".to_string();
        cfg.annotate.block_templates.add.end = "// end add".to_string();
        cfg.annotate.block_templates.modify.start = "// modify {reason}".to_string();
        cfg.annotate.block_templates.modify.end = "// end modify".to_string();
        cfg.annotate.block_templates.del.start = "// del {reason}".to_string();
        cfg.annotate.block_templates.del.end = "// end del".to_string();
        cfg.annotate.old_code.mode = Some(AnnotateOldCodeMode::None);

        let baseline = "";
        let current =
            "// add first\n// add first\nint a = 1;\n// end add\nint b = 2;\n// end add\n";
        let normalized =
            normalize_content_before_render(baseline, current, &cfg, "demo.c").unwrap();
        assert_eq!(normalized.logical_content, "int a = 1;\nint b = 2;\n");
        assert_eq!(normalized.segments.len(), 1);
        assert_eq!(normalized.segments[0].kind, ChangeKind::Add);

        let rendered = apply_c_line_segments(
            &normalized.logical_content,
            &normalized.segments,
            &RuntimeContext {
                reason: "final".to_string(),
                reference_kind: "bug".to_string(),
                reference_value: "ID-1".to_string(),
                author_tag: "QA".to_string(),
                date: "2026-04-01".to_string(),
            },
            &cfg,
            "demo.c",
        );
        assert_eq!(rendered.matches("// add final").count(), 1);
        assert_eq!(rendered.matches("// end add").count(), 1);
    }

    #[test]
    fn pending_modify_and_delete_blocks_restore_and_rebuild() {
        let mut cfg = AppConfig::default();
        cfg.annotate.block_templates.add.start = "// add {reason}".to_string();
        cfg.annotate.block_templates.add.end = "// end add".to_string();
        cfg.annotate.block_templates.modify.start = "// modify {reason}".to_string();
        cfg.annotate.block_templates.modify.end = "// end modify".to_string();
        cfg.annotate.block_templates.del.start = "// del {reason}".to_string();
        cfg.annotate.block_templates.del.end = "// end del".to_string();
        cfg.annotate.old_code.mode = None;

        let baseline_modify = "int a = 1;\n";
        let current_modify = "// modify old\n// old:\n//   int a = 1;\nint a = 3;\n// end modify\n";
        let normalized_modify =
            normalize_content_before_render(baseline_modify, current_modify, &cfg, "demo.c")
                .unwrap();
        assert_eq!(normalized_modify.logical_content, "int a = 3;\n");
        assert_eq!(normalized_modify.segments.len(), 1);
        assert_eq!(normalized_modify.segments[0].kind, ChangeKind::Modify);

        let baseline_delete = "int a = 1;\n";
        let current_delete = "// del old\n// old:\n//   int a = 1;\n// end del\n";
        let normalized_delete =
            normalize_content_before_render(baseline_delete, current_delete, &cfg, "demo.c")
                .unwrap();
        assert_eq!(normalized_delete.logical_content, "");
        assert_eq!(normalized_delete.segments.len(), 1);
        assert_eq!(normalized_delete.segments[0].kind, ChangeKind::Delete);
    }

    #[test]
    fn history_annotation_blocks_are_not_rolled_back() {
        let mut cfg = AppConfig::default();
        cfg.annotate.block_templates.add.start = "// add {reason}".to_string();
        cfg.annotate.block_templates.add.end = "// end add".to_string();
        cfg.annotate.block_templates.modify.start = "// modify {reason}".to_string();
        cfg.annotate.block_templates.modify.end = "// end modify".to_string();
        cfg.annotate.block_templates.del.start = "// del {reason}".to_string();
        cfg.annotate.block_templates.del.end = "// end del".to_string();
        cfg.annotate.old_code.mode = Some(AnnotateOldCodeMode::None);

        let baseline = "// add old\nint a = 1;\n// end add\n";
        let current = "// add old\nint a = 1;\nint b = 2;\n// end add\n";
        let normalized =
            normalize_content_before_render(baseline, current, &cfg, "demo.c").unwrap();
        assert_eq!(normalized.logical_content, current);
        assert_eq!(normalized.segments.len(), 1);
        assert_eq!(normalized.segments[0].kind, ChangeKind::Add);
        assert_eq!(
            normalized.segments[0].new_lines,
            vec!["int b = 2;".to_string()]
        );
    }

    #[test]
    fn reusable_context_defaults_require_unique_consensus() {
        let accepted = resolve_reusable_context_defaults(&[
            ContextReuseCandidate {
                reason: Some("sync".to_string()),
                reference_kind: Some("bug".to_string()),
                reference_value: Some("ID-7".to_string()),
            },
            ContextReuseCandidate {
                reason: Some("sync".to_string()),
                reference_kind: Some("bug".to_string()),
                reference_value: Some("ID-7".to_string()),
            },
        ]);
        assert!(accepted.is_some());

        let conflicted = resolve_reusable_context_defaults(&[
            ContextReuseCandidate {
                reason: Some("sync".to_string()),
                reference_kind: Some("bug".to_string()),
                reference_value: Some("ID-7".to_string()),
            },
            ContextReuseCandidate {
                reason: Some("another".to_string()),
                reference_kind: Some("bug".to_string()),
                reference_value: Some("ID-7".to_string()),
            },
        ]);
        assert!(conflicted.is_none());
    }

    #[test]
    fn context_extraction_recovers_kind_when_reason_contains_spaces() {
        let mut cfg = AppConfig::default();
        cfg.annotate.reference_kinds = vec!["bug".to_string(), "req".to_string()];
        cfg.annotate.block_templates.del.start =
            "// del by {author_tag} {date} for {reason} {reference_kind}:{reference_value} {@"
                .to_string();
        cfg.annotate.block_templates.del.end = "//@}".to_string();
        cfg.annotate.old_code.mode = Some(AnnotateOldCodeMode::None);

        let current =
            "// del by jingd 2026-04-01 for test delete code by commit req:1234567 {@\n//@}\n";
        let normalized = normalize_content_before_render("", current, &cfg, "demo.c").unwrap();
        let defaults = resolve_reusable_context_defaults(&normalized.context_candidates)
            .expect("expected reusable context");
        assert_eq!(defaults.reason, "test delete code by commit");
        assert_eq!(defaults.reference_kind, "req");
        assert_eq!(defaults.reference_value, "1234567");
    }

    #[test]
    fn malformed_block_boundary_fails_conservatively() {
        let mut cfg = AppConfig::default();
        cfg.annotate.block_templates.add.start = "// add {reason}".to_string();
        cfg.annotate.block_templates.add.end = "// end add".to_string();
        cfg.annotate.old_code.mode = Some(AnnotateOldCodeMode::None);
        let baseline = "";
        let malformed = "// add missing-end\nint a = 1;\n";
        let result = normalize_content_before_render(baseline, malformed, &cfg, "demo.c");
        assert!(result.is_err());
    }

    #[test]
    fn overlapping_pending_blocks_fail_conservatively() {
        let mut cfg = AppConfig::default();
        cfg.annotate.block_templates.add.start = "// add {reason}".to_string();
        cfg.annotate.block_templates.add.end = "// end add".to_string();
        cfg.annotate.block_templates.modify.start = "// modify {reason}".to_string();
        cfg.annotate.block_templates.modify.end = "// end modify".to_string();
        cfg.annotate.old_code.mode = Some(AnnotateOldCodeMode::None);

        let baseline = "";
        let overlapping = "// add outer\n// modify inner\nint a = 1;\n// end add\n// end modify\n";
        let result = normalize_content_before_render(baseline, overlapping, &cfg, "demo.c");
        assert!(result.is_err());
    }

    #[test]
    fn latest_commit_baseline_reads_head_parent() {
        let repo = init_test_repo();
        fs::write(repo.path().join("demo.c"), "int a = 1;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "init"]);

        fs::write(repo.path().join("demo.c"), "int a = 2;\n").unwrap();
        run_git(repo.path(), &["add", "demo.c"]);
        run_git(repo.path(), &["commit", "-m", "update"]);

        let baseline = load_baseline_content(
            repo.path(),
            true,
            &FileChange {
                path: "demo.c".to_string(),
                kind: ChangeKind::Modify,
                from_untracked: false,
            },
        )
        .unwrap();
        assert_eq!(baseline, "int a = 1;\n");
    }

    fn init_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        run_git(dir.path(), &["init"]);
        run_git(dir.path(), &["config", "user.email", "xgit@test.local"]);
        run_git(dir.path(), &["config", "user.name", "xgit-test"]);
        fs::create_dir_all(dir.path().join(".xgit")).unwrap();
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
