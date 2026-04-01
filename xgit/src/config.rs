use crate::i18n;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_CONFIG_TOML: &str = include_str!("../config/default.toml");

#[derive(Debug, Clone, Default)]
pub struct LoadConfigOptions;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub effective: AppConfig,
    pub global_path: PathBuf,
    pub project_path: Option<PathBuf>,
    pub git_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub ui: UiConfig,
    pub features: FeaturesConfig,
    pub push: PushConfig,
    pub annotate: AnnotateConfig,
    pub identity: IdentityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UiConfig {
    pub lang: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FeaturesConfig {
    pub push: bool,
    pub annotate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct PushConfig {
    pub placeholder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AnnotateConfig {
    pub staged: StagedConfig,
    pub form: AnnotateFormConfig,
    pub reference_kinds: Vec<String>,
    pub render: AnnotateRenderConfig,
    pub block_templates: BlockTemplates,
    pub file_rules: Vec<FileRuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct StagedConfig {
    pub include_untracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AnnotateFormConfig {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AnnotateRenderConfig {
    pub align_with_code_indent: bool,
    pub wrap_blank_lines: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct BlockTemplates {
    pub add: BlockTemplate,
    pub modify: BlockTemplate,
    pub del: BlockTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct BlockTemplate {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FileRuleConfig {
    pub pattern: String,
    pub renderer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct IdentityConfig {
    pub author_tag: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ui: UiConfig::default(),
            features: FeaturesConfig::default(),
            push: PushConfig::default(),
            annotate: AnnotateConfig::default(),
            identity: IdentityConfig::default(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            lang: "zh-CN".to_string(),
        }
    }
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            push: true,
            annotate: true,
        }
    }
}

impl Default for AnnotateConfig {
    fn default() -> Self {
        Self {
            staged: StagedConfig::default(),
            form: AnnotateFormConfig::default(),
            reference_kinds: vec!["bug".to_string(), "req".to_string()],
            render: AnnotateRenderConfig::default(),
            block_templates: BlockTemplates::default(),
            file_rules: vec![
                FileRuleConfig {
                    pattern: "*.c".to_string(),
                    renderer: "c_line_block".to_string(),
                },
                FileRuleConfig {
                    pattern: "*.h".to_string(),
                    renderer: "c_line_block".to_string(),
                },
                FileRuleConfig {
                    pattern: "*.cpp".to_string(),
                    renderer: "c_line_block".to_string(),
                },
                FileRuleConfig {
                    pattern: "*.java".to_string(),
                    renderer: "c_line_block".to_string(),
                },
            ],
        }
    }
}

impl Default for StagedConfig {
    fn default() -> Self {
        Self {
            include_untracked: false,
        }
    }
}

impl Default for AnnotateFormConfig {
    fn default() -> Self {
        Self {
            fields: vec![
                "reason".to_string(),
                "reference_kind".to_string(),
                "reference_value".to_string(),
            ],
        }
    }
}

impl Default for AnnotateRenderConfig {
    fn default() -> Self {
        Self {
            align_with_code_indent: false,
            wrap_blank_lines: true,
        }
    }
}

impl Default for BlockTemplates {
    fn default() -> Self {
        Self {
            add: BlockTemplate {
                start: "// {author_tag} {date} add: {reason} ({reference_kind}:{reference_value})"
                    .to_string(),
                end: compatibility_end_template("add"),
            },
            modify: BlockTemplate {
                start: "// {author_tag} {date} modify: {reason} ({reference_kind}:{reference_value}) old={old}"
                    .to_string(),
                end: compatibility_end_template("modify"),
            },
            del: BlockTemplate {
                start: "// {author_tag} {date} del: {reason} ({reference_kind}:{reference_value}) old={old}"
                    .to_string(),
                end: compatibility_end_template("del"),
            },
        }
    }
}

impl Default for BlockTemplate {
    fn default() -> Self {
        Self {
            start: String::new(),
            end: String::new(),
        }
    }
}

impl Default for FileRuleConfig {
    fn default() -> Self {
        Self {
            pattern: "*.c".to_string(),
            renderer: "c_line_block".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAppConfig {
    ui: Option<PartialUiConfig>,
    features: Option<PartialFeaturesConfig>,
    push: Option<PushConfig>,
    annotate: Option<PartialAnnotateConfig>,
    identity: Option<PartialIdentityConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialUiConfig {
    lang: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialFeaturesConfig {
    push: Option<bool>,
    annotate: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAnnotateConfig {
    staged: Option<PartialStagedConfig>,
    form: Option<PartialAnnotateFormConfig>,
    reference_kinds: Option<Vec<String>>,
    render: Option<PartialAnnotateRenderConfig>,
    block_templates: Option<PartialBlockTemplates>,
    policies: Option<PartialPolicyTemplates>,
    file_rules: Option<Vec<FileRuleConfig>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialStagedConfig {
    include_untracked: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAnnotateFormConfig {
    fields: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAnnotateRenderConfig {
    align_with_code_indent: Option<bool>,
    wrap_blank_lines: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialPolicyTemplates {
    add: Option<String>,
    modify: Option<String>,
    del: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialBlockTemplates {
    add: Option<PartialBlockTemplate>,
    modify: Option<PartialBlockTemplate>,
    del: Option<PartialBlockTemplate>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialBlockTemplate {
    start: Option<String>,
    end: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialIdentityConfig {
    author_tag: Option<String>,
    name: Option<String>,
    email: Option<String>,
}

impl AppConfig {
    fn apply_partial(&mut self, partial: PartialAppConfig) {
        if let Some(ui) = partial.ui {
            if let Some(lang) = ui.lang {
                self.ui.lang = lang;
            }
        }
        if let Some(features) = partial.features {
            if let Some(push) = features.push {
                self.features.push = push;
            }
            if let Some(annotate) = features.annotate {
                self.features.annotate = annotate;
            }
        }
        if let Some(push) = partial.push {
            self.push = push;
        }
        if let Some(annotate) = partial.annotate {
            if let Some(staged) = annotate.staged {
                if let Some(include_untracked) = staged.include_untracked {
                    self.annotate.staged.include_untracked = include_untracked;
                }
            }
            if let Some(form) = annotate.form {
                if let Some(fields) = form.fields {
                    self.annotate.form.fields = fields;
                }
            }
            if let Some(reference_kinds) = annotate.reference_kinds {
                self.annotate.reference_kinds = reference_kinds;
            }
            if let Some(render) = annotate.render {
                if let Some(align_with_code_indent) = render.align_with_code_indent {
                    self.annotate.render.align_with_code_indent = align_with_code_indent;
                }
                if let Some(wrap_blank_lines) = render.wrap_blank_lines {
                    self.annotate.render.wrap_blank_lines = wrap_blank_lines;
                }
            }
            let mut has_new_add_template = false;
            let mut has_new_modify_template = false;
            let mut has_new_del_template = false;
            if let Some(block_templates) = annotate.block_templates {
                if let Some(add) = block_templates.add {
                    has_new_add_template = true;
                    if let Some(start) = add.start {
                        self.annotate.block_templates.add.start = start;
                    }
                    if let Some(end) = add.end {
                        self.annotate.block_templates.add.end = end;
                    }
                }
                if let Some(modify) = block_templates.modify {
                    has_new_modify_template = true;
                    if let Some(start) = modify.start {
                        self.annotate.block_templates.modify.start = start;
                    }
                    if let Some(end) = modify.end {
                        self.annotate.block_templates.modify.end = end;
                    }
                }
                if let Some(del) = block_templates.del {
                    has_new_del_template = true;
                    if let Some(start) = del.start {
                        self.annotate.block_templates.del.start = start;
                    }
                    if let Some(end) = del.end {
                        self.annotate.block_templates.del.end = end;
                    }
                }
            }
            if let Some(policies) = annotate.policies {
                if !has_new_add_template {
                    if let Some(add) = policies.add {
                        self.annotate.block_templates.add.start =
                            legacy_policy_to_start_template(&add);
                        self.annotate.block_templates.add.end = compatibility_end_template("add");
                    }
                }
                if !has_new_modify_template {
                    if let Some(modify) = policies.modify {
                        self.annotate.block_templates.modify.start =
                            legacy_policy_to_start_template(&modify);
                        self.annotate.block_templates.modify.end =
                            compatibility_end_template("modify");
                    }
                }
                if !has_new_del_template {
                    if let Some(del) = policies.del {
                        self.annotate.block_templates.del.start =
                            legacy_policy_to_start_template(&del);
                        self.annotate.block_templates.del.end = compatibility_end_template("del");
                    }
                }
            }
            if let Some(file_rules) = annotate.file_rules {
                self.annotate.file_rules = file_rules;
            }
        }
        if let Some(identity) = partial.identity {
            if let Some(author_tag) = identity.author_tag {
                self.identity.author_tag = Some(author_tag);
            }
            if let Some(name) = identity.name {
                self.identity.name = Some(name);
            }
            if let Some(email) = identity.email {
                self.identity.email = Some(email);
            }
        }
    }
}

pub fn load_runtime_config(cwd: &Path, _opts: &LoadConfigOptions) -> Result<RuntimeConfig> {
    let global_path = global_config_path()?;
    let git_root = resolve_git_root(cwd).ok();
    let project_path = git_root
        .as_ref()
        .map(|root| project_config_path(root.as_path()));

    let global_str = read_optional_file(&global_path)?;
    let project_str = if let Some(path) = &project_path {
        read_optional_file(path)?
    } else {
        None
    };

    let env_map: HashMap<String, String> = std::env::vars().collect();
    let cfg = merge_layers(
        global_str.as_deref(),
        project_str.as_deref(),
        &env_map,
        i18n::detect_system_locale(),
    )?;

    Ok(RuntimeConfig {
        effective: cfg,
        global_path,
        project_path,
        git_root,
    })
}

pub fn merge_layers(
    global_toml: Option<&str>,
    project_toml: Option<&str>,
    env_map: &HashMap<String, String>,
    system_locale: &str,
) -> Result<AppConfig> {
    let mut cfg = load_default_config()?;

    if let Some(raw) = global_toml {
        cfg.apply_partial(parse_partial(raw).context("invalid global config")?);
    }
    if let Some(raw) = project_toml {
        cfg.apply_partial(parse_partial(raw).context("invalid project config")?);
    }

    cfg.apply_partial(parse_env_partial(env_map)?);

    cfg.ui.lang =
        i18n::resolve_locale(env_map.get("XGIT_LANG").cloned(), Some(cfg.ui.lang.clone()));

    if i18n::normalize_locale(Some(cfg.ui.lang.as_str())).is_none() {
        cfg.ui.lang = i18n::normalize_locale(Some(system_locale))
            .unwrap_or("zh-CN")
            .to_string();
    }

    Ok(cfg)
}

pub fn global_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("unable to determine home directory"))?;
    Ok(home.join(".xgit").join("config.toml"))
}

pub fn project_config_path(git_root: &Path) -> PathBuf {
    git_root.join(".xgit").join("config.toml")
}

pub fn ensure_config_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory: {}", parent.display()))?;
    }
    Ok(())
}

pub fn save_config(path: &Path, config: &AppConfig) -> Result<()> {
    ensure_config_parent(path)?;
    let raw = toml::to_string_pretty(config).context("failed to serialize config")?;
    std::fs::write(path, raw)
        .with_context(|| format!("failed to write config file: {}", path.display()))?;
    Ok(())
}

pub fn resolve_git_root(cwd: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git rev-parse --show-toplevel")?;
    if !output.status.success() {
        return Err(anyhow!("not in git workspace"));
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        return Err(anyhow!("git workspace root is empty"));
    }
    Ok(PathBuf::from(root))
}

fn load_default_config() -> Result<AppConfig> {
    let mut cfg = AppConfig::default();
    cfg.apply_partial(
        parse_partial(DEFAULT_CONFIG_TOML).context("invalid built-in default config")?,
    );
    Ok(cfg)
}

fn parse_partial(raw: &str) -> Result<PartialAppConfig> {
    Ok(toml::from_str::<PartialAppConfig>(raw)?)
}

fn read_optional_file(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    Ok(Some(content))
}

fn parse_env_partial(env_map: &HashMap<String, String>) -> Result<PartialAppConfig> {
    let mut partial = PartialAppConfig::default();

    if let Some(lang) = env_map.get("XGIT_LANG") {
        partial.ui = Some(PartialUiConfig {
            lang: Some(lang.to_string()),
        });
    }

    let mut features = PartialFeaturesConfig::default();
    if let Some(value) = env_map.get("XGIT_FEATURE_PUSH") {
        features.push = Some(parse_bool_env("XGIT_FEATURE_PUSH", value)?);
    }
    if let Some(value) = env_map.get("XGIT_FEATURE_ANNOTATE") {
        features.annotate = Some(parse_bool_env("XGIT_FEATURE_ANNOTATE", value)?);
    }
    if features.push.is_some() || features.annotate.is_some() {
        partial.features = Some(features);
    }

    if let Some(value) = env_map.get("XGIT_ANNOTATE_INCLUDE_UNTRACKED") {
        partial.annotate = Some(PartialAnnotateConfig {
            staged: Some(PartialStagedConfig {
                include_untracked: Some(parse_bool_env("XGIT_ANNOTATE_INCLUDE_UNTRACKED", value)?),
            }),
            form: None,
            reference_kinds: None,
            render: None,
            block_templates: None,
            policies: None,
            file_rules: None,
        });
    }

    Ok(partial)
}

fn parse_bool_env(name: &str, value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(anyhow!("invalid boolean env {}={}", name, value)),
    }
}

fn compatibility_end_template(kind: &str) -> String {
    format!("// end {kind}")
}

fn legacy_policy_to_start_template(policy: &str) -> String {
    if policy.is_empty() {
        return "//".to_string();
    }
    policy
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                "//".to_string()
            } else {
                format!("// {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{merge_layers, project_config_path, resolve_git_root};
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn project_overrides_global() {
        let global = r#"
[features]
push = true
"#;
        let project = r#"
[features]
push = false
"#;
        let env = HashMap::new();
        let cfg = merge_layers(Some(global), Some(project), &env, "zh-CN").unwrap();
        assert!(!cfg.features.push);
    }

    #[test]
    fn env_overrides_file() {
        let global = r#"
[features]
annotate = true
"#;
        let mut env = HashMap::new();
        env.insert("XGIT_FEATURE_ANNOTATE".to_string(), "false".to_string());
        let cfg = merge_layers(Some(global), None, &env, "zh-CN").unwrap();
        assert!(!cfg.features.annotate);
    }

    #[test]
    fn env_overrides_config_for_locale() {
        let global = r#"
[ui]
lang = "zh-CN"
"#;
        let mut env = HashMap::new();
        env.insert("XGIT_LANG".to_string(), "en-US".to_string());
        let cfg = merge_layers(Some(global), None, &env, "zh-CN").unwrap();
        assert_eq!(cfg.ui.lang, "en-US");
    }

    #[test]
    fn project_scope_requires_git_workspace() {
        let tmp = TempDir::new().unwrap();
        let err = resolve_git_root(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("not in git workspace"));
    }

    #[test]
    fn project_scope_path_uses_git_root() {
        let root = std::path::PathBuf::from("/tmp/repo");
        let expected = std::path::PathBuf::from("/tmp/repo/.xgit/config.toml");
        assert_eq!(project_config_path(&root), expected);
    }

    #[test]
    fn render_options_can_be_overridden_by_layers() {
        let global = r#"
[annotate.render]
align_with_code_indent = true
wrap_blank_lines = false
"#;
        let env = HashMap::new();
        let cfg = merge_layers(Some(global), None, &env, "zh-CN").unwrap();
        assert!(cfg.annotate.render.align_with_code_indent);
        assert!(!cfg.annotate.render.wrap_blank_lines);
    }

    #[test]
    fn legacy_policies_fallback_to_block_templates() {
        let project = r#"
[annotate.policies]
add = "legacy add {author_tag}"
"#;
        let env = HashMap::new();
        let cfg = merge_layers(None, Some(project), &env, "zh-CN").unwrap();

        assert_eq!(
            cfg.annotate.block_templates.add.start,
            "// legacy add {author_tag}"
        );
        assert_eq!(cfg.annotate.block_templates.add.end, "// end add");
    }

    #[test]
    fn block_templates_take_precedence_over_legacy_policies() {
        let project = r#"
[annotate.policies]
add = "legacy add"

[annotate.block_templates.add]
start = "// custom add"
end = "//@}"
"#;
        let env = HashMap::new();
        let cfg = merge_layers(None, Some(project), &env, "zh-CN").unwrap();

        assert_eq!(cfg.annotate.block_templates.add.start, "// custom add");
        assert_eq!(cfg.annotate.block_templates.add.end, "//@}");
    }
}
