use crate::code_file_types::builtin_default_file_rules;
use crate::i18n;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
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
    pub reset: bool,
    pub checkout_remote: bool,
    pub completion: bool,
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
    pub date: AnnotateDateConfig,
    pub render: AnnotateRenderConfig,
    pub old_code: AnnotateOldCodeConfig,
    pub block_templates: BlockTemplates,
    pub file_rules: Vec<FileRuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct StagedConfig {
    pub include_untracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AnnotateFormConfig {
    pub fields: Vec<AnnotateFormFieldConfig>,
    pub option_sets: BTreeMap<String, AnnotateOptionSetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct AnnotateFormFieldConfig {
    pub id: String,
    pub label: String,
    pub kind: AnnotateFormFieldKind,
    pub required: bool,
    pub option_set: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct AnnotateOptionSetConfig {
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnnotateFormFieldKind {
    #[default]
    Text,
    SingleSelect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AnnotateRenderConfig {
    pub align_with_code_indent: bool,
    pub wrap_blank_lines: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AnnotateDateConfig {
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AnnotateOldCodeConfig {
    pub enabled: bool,
    pub mode: Option<AnnotateOldCodeMode>,
    pub line_comment: AnnotateOldCodeLineCommentConfig,
    pub block_comment: AnnotateOldCodeBlockCommentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnnotateOldCodeMode {
    None,
    LineComment,
    BlockComment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AnnotateOldCodeLineCommentConfig {
    pub layout: AnnotateOldCodeLineLayout,
    pub header: String,
    pub body_prefix: String,
    pub body_suffix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnnotateOldCodeLineLayout {
    #[default]
    PerLine,
    HeaderBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AnnotateOldCodeBlockCommentConfig {
    pub title: String,
    pub body_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct BlockTemplates {
    pub add: BlockTemplate,
    pub modify: BlockTemplate,
    pub del: BlockTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct BlockTemplate {
    pub enabled: bool,
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
            reset: true,
            checkout_remote: true,
            completion: true,
        }
    }
}

impl Default for AnnotateConfig {
    fn default() -> Self {
        Self {
            staged: StagedConfig::default(),
            form: AnnotateFormConfig::default(),
            date: AnnotateDateConfig::default(),
            render: AnnotateRenderConfig::default(),
            old_code: AnnotateOldCodeConfig::default(),
            block_templates: BlockTemplates::default(),
            file_rules: builtin_default_file_rules(),
        }
    }
}

impl Default for AnnotateFormConfig {
    fn default() -> Self {
        Self {
            fields: default_annotate_form_fields(),
            option_sets: default_annotate_option_sets(),
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

impl Default for AnnotateDateConfig {
    fn default() -> Self {
        Self {
            format: "yyyy-mm-dd".to_string(),
        }
    }
}

impl Default for AnnotateOldCodeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: None,
            line_comment: AnnotateOldCodeLineCommentConfig::default(),
            block_comment: AnnotateOldCodeBlockCommentConfig::default(),
        }
    }
}

impl Default for AnnotateOldCodeLineCommentConfig {
    fn default() -> Self {
        Self {
            layout: AnnotateOldCodeLineLayout::PerLine,
            header: "old:".to_string(),
            body_prefix: "old: ".to_string(),
            body_suffix: String::new(),
        }
    }
}

impl Default for AnnotateOldCodeBlockCommentConfig {
    fn default() -> Self {
        Self {
            title: "cover old codes".to_string(),
            body_prefix: String::new(),
        }
    }
}

impl Default for BlockTemplates {
    fn default() -> Self {
        Self {
            add: default_block_template("add"),
            modify: default_block_template("modify"),
            del: default_block_template("del"),
        }
    }
}

impl BlockTemplate {
    pub fn enable_if_customized_for(&mut self, kind: &str) {
        if self.start != builtin_start_template(kind)
            || self.end != compatibility_end_template(kind)
        {
            self.enabled = true;
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

impl AnnotateConfig {
    pub fn reference_kind_option_set_name() -> &'static str {
        "reference_kinds"
    }

    pub fn reference_kind_values(&self) -> &[String] {
        self.form
            .option_values(Self::reference_kind_option_set_name())
    }

    #[cfg(test)]
    pub fn reference_kind_values_mut(&mut self) -> &mut Vec<String> {
        &mut self
            .form
            .option_sets
            .entry(Self::reference_kind_option_set_name().to_string())
            .or_insert_with(|| AnnotateOptionSetConfig {
                values: vec!["bug".to_string(), "req".to_string()],
            })
            .values
    }

    pub fn normalize(&mut self) {
        self.form.normalize();
    }
}

impl AnnotateFormConfig {
    pub fn uses_field(&self, id: &str) -> bool {
        self.fields.iter().any(|field| field.id == id)
    }

    pub fn option_values(&self, name: &str) -> &[String] {
        self.option_sets
            .get(name)
            .map(|set| set.values.as_slice())
            .unwrap_or(&[])
    }

    pub fn normalize(&mut self) {
        self.fields = self
            .fields
            .iter()
            .filter_map(|field| {
                let id = field.id.trim();
                if id.is_empty() {
                    None
                } else {
                    Some(AnnotateFormFieldConfig {
                        id: id.to_string(),
                        label: field.label.trim().to_string(),
                        kind: field.kind.clone(),
                        required: field.required,
                        option_set: field
                            .option_set
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(std::string::ToString::to_string),
                    })
                }
            })
            .collect();

        if self.fields.is_empty() {
            self.fields = default_annotate_form_fields();
        }

        self.option_sets
            .entry(AnnotateConfig::reference_kind_option_set_name().to_string())
            .or_insert_with(|| AnnotateOptionSetConfig {
                values: vec!["bug".to_string(), "req".to_string()],
            });

        self.option_sets.retain(|name, set| {
            let trimmed_name = name.trim();
            if trimmed_name.is_empty() {
                return false;
            }

            let mut values = Vec::<String>::new();
            for value in &set.values {
                let trimmed_value = value.trim();
                if trimmed_value.is_empty()
                    || values.iter().any(|existing| existing == trimmed_value)
                {
                    continue;
                }
                values.push(trimmed_value.to_string());
            }
            set.values = values;
            true
        });

        for field in &mut self.fields {
            if field.label.trim().is_empty() {
                field.label = default_field_label(field.id.as_str());
            }
            if field.id == "reference_kind" {
                field.kind = AnnotateFormFieldKind::SingleSelect;
                if field.option_set.is_none() {
                    field.option_set =
                        Some(AnnotateConfig::reference_kind_option_set_name().to_string());
                }
            }
        }
    }
}

fn default_annotate_form_fields() -> Vec<AnnotateFormFieldConfig> {
    vec![
        default_field_definition("reason"),
        default_field_definition("reference_kind"),
        default_field_definition("reference_value"),
    ]
}

fn default_annotate_option_sets() -> BTreeMap<String, AnnotateOptionSetConfig> {
    BTreeMap::from([(
        AnnotateConfig::reference_kind_option_set_name().to_string(),
        AnnotateOptionSetConfig {
            values: vec!["bug".to_string(), "req".to_string()],
        },
    )])
}

pub fn default_field_definition(id: &str) -> AnnotateFormFieldConfig {
    match id {
        "reason" => AnnotateFormFieldConfig {
            id: "reason".to_string(),
            label: "原因".to_string(),
            kind: AnnotateFormFieldKind::Text,
            required: true,
            option_set: None,
        },
        "reference_kind" => AnnotateFormFieldConfig {
            id: "reference_kind".to_string(),
            label: "引用类型".to_string(),
            kind: AnnotateFormFieldKind::SingleSelect,
            required: true,
            option_set: Some(AnnotateConfig::reference_kind_option_set_name().to_string()),
        },
        "reference_value" => AnnotateFormFieldConfig {
            id: "reference_value".to_string(),
            label: "引用值".to_string(),
            kind: AnnotateFormFieldKind::Text,
            required: true,
            option_set: None,
        },
        other => AnnotateFormFieldConfig {
            id: other.to_string(),
            label: default_field_label(other),
            kind: AnnotateFormFieldKind::Text,
            required: true,
            option_set: None,
        },
    }
}

fn default_field_label(id: &str) -> String {
    match id {
        "reason" => "原因".to_string(),
        "reference_kind" => "引用类型".to_string(),
        "reference_value" => "引用值".to_string(),
        other => other.replace('_', " "),
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
    reset: Option<bool>,
    checkout_remote: Option<bool>,
    completion: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAnnotateConfig {
    staged: Option<PartialStagedConfig>,
    form: Option<PartialAnnotateFormConfig>,
    reference_kinds: Option<Vec<String>>,
    date: Option<PartialAnnotateDateConfig>,
    render: Option<PartialAnnotateRenderConfig>,
    old_code: Option<PartialAnnotateOldCodeConfig>,
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
    fields: Option<PartialAnnotateFormFields>,
    option_sets: Option<BTreeMap<String, AnnotateOptionSetConfig>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum PartialAnnotateFormFields {
    Legacy(Vec<String>),
    Structured(Vec<AnnotateFormFieldConfig>),
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAnnotateRenderConfig {
    align_with_code_indent: Option<bool>,
    wrap_blank_lines: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAnnotateDateConfig {
    format: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAnnotateOldCodeConfig {
    enabled: Option<bool>,
    mode: Option<AnnotateOldCodeMode>,
    line_comment: Option<PartialAnnotateOldCodeLineCommentConfig>,
    block_comment: Option<PartialAnnotateOldCodeBlockCommentConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAnnotateOldCodeLineCommentConfig {
    layout: Option<AnnotateOldCodeLineLayout>,
    header: Option<String>,
    body_prefix: Option<String>,
    body_suffix: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct PartialAnnotateOldCodeBlockCommentConfig {
    title: Option<String>,
    body_prefix: Option<String>,
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
    enabled: Option<bool>,
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
            if let Some(reset) = features.reset {
                self.features.reset = reset;
            }
            if let Some(checkout_remote) = features.checkout_remote {
                self.features.checkout_remote = checkout_remote;
            }
            if let Some(completion) = features.completion {
                self.features.completion = completion;
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
                    self.annotate.form.fields = match fields {
                        PartialAnnotateFormFields::Legacy(ids) => ids
                            .into_iter()
                            .map(|id| default_field_definition(id.as_str()))
                            .collect(),
                        PartialAnnotateFormFields::Structured(fields) => fields,
                    };
                }
                if let Some(option_sets) = form.option_sets {
                    for (name, option_set) in option_sets {
                        self.annotate.form.option_sets.insert(name, option_set);
                    }
                }
            }
            if let Some(reference_kinds) = annotate.reference_kinds {
                self.annotate.form.option_sets.insert(
                    AnnotateConfig::reference_kind_option_set_name().to_string(),
                    AnnotateOptionSetConfig {
                        values: reference_kinds,
                    },
                );
            }
            if let Some(date) = annotate.date {
                if let Some(format) = date.format {
                    self.annotate.date.format = format;
                }
            }
            if let Some(render) = annotate.render {
                if let Some(align_with_code_indent) = render.align_with_code_indent {
                    self.annotate.render.align_with_code_indent = align_with_code_indent;
                }
                if let Some(wrap_blank_lines) = render.wrap_blank_lines {
                    self.annotate.render.wrap_blank_lines = wrap_blank_lines;
                }
            }
            if let Some(old_code) = annotate.old_code {
                if let Some(enabled) = old_code.enabled {
                    self.annotate.old_code.enabled = enabled;
                }
                if let Some(mode) = old_code.mode {
                    if mode == AnnotateOldCodeMode::None {
                        self.annotate.old_code.enabled = false;
                    } else {
                        self.annotate.old_code.mode = Some(mode);
                    }
                }
                if let Some(line_comment) = old_code.line_comment {
                    if let Some(layout) = line_comment.layout {
                        self.annotate.old_code.line_comment.layout = layout;
                    }
                    if let Some(header) = line_comment.header {
                        self.annotate.old_code.line_comment.header = header;
                    }
                    if let Some(body_prefix) = line_comment.body_prefix {
                        self.annotate.old_code.line_comment.body_prefix = body_prefix;
                    }
                    if let Some(body_suffix) = line_comment.body_suffix {
                        self.annotate.old_code.line_comment.body_suffix = body_suffix;
                    }
                }
                if let Some(block_comment) = old_code.block_comment {
                    if let Some(title) = block_comment.title {
                        self.annotate.old_code.block_comment.title = title;
                    }
                    if let Some(body_prefix) = block_comment.body_prefix {
                        self.annotate.old_code.block_comment.body_prefix = body_prefix;
                    }
                }
            }
            let mut has_new_add_template = false;
            let mut has_new_modify_template = false;
            let mut has_new_del_template = false;
            if let Some(block_templates) = annotate.block_templates {
                if let Some(add) = block_templates.add {
                    has_new_add_template = true;
                    if let Some(enabled) = add.enabled {
                        self.annotate.block_templates.add.enabled = enabled;
                    }
                    if let Some(start) = add.start {
                        self.annotate.block_templates.add.start = start;
                    }
                    if let Some(end) = add.end {
                        self.annotate.block_templates.add.end = end;
                    }
                    if add.enabled.is_none() {
                        self.annotate
                            .block_templates
                            .add
                            .enable_if_customized_for("add");
                    }
                }
                if let Some(modify) = block_templates.modify {
                    has_new_modify_template = true;
                    if let Some(enabled) = modify.enabled {
                        self.annotate.block_templates.modify.enabled = enabled;
                    }
                    if let Some(start) = modify.start {
                        self.annotate.block_templates.modify.start = start;
                    }
                    if let Some(end) = modify.end {
                        self.annotate.block_templates.modify.end = end;
                    }
                    if modify.enabled.is_none() {
                        self.annotate
                            .block_templates
                            .modify
                            .enable_if_customized_for("modify");
                    }
                }
                if let Some(del) = block_templates.del {
                    has_new_del_template = true;
                    if let Some(enabled) = del.enabled {
                        self.annotate.block_templates.del.enabled = enabled;
                    }
                    if let Some(start) = del.start {
                        self.annotate.block_templates.del.start = start;
                    }
                    if let Some(end) = del.end {
                        self.annotate.block_templates.del.end = end;
                    }
                    if del.enabled.is_none() {
                        self.annotate
                            .block_templates
                            .del
                            .enable_if_customized_for("del");
                    }
                }
            }
            if let Some(policies) = annotate.policies {
                if !has_new_add_template {
                    if let Some(add) = policies.add {
                        self.annotate.block_templates.add.enabled = true;
                        self.annotate.block_templates.add.start =
                            legacy_policy_to_start_template(&add);
                        self.annotate.block_templates.add.end = compatibility_end_template("add");
                    }
                }
                if !has_new_modify_template {
                    if let Some(modify) = policies.modify {
                        self.annotate.block_templates.modify.enabled = true;
                        self.annotate.block_templates.modify.start =
                            legacy_policy_to_start_template(&modify);
                        self.annotate.block_templates.modify.end =
                            compatibility_end_template("modify");
                    }
                }
                if !has_new_del_template {
                    if let Some(del) = policies.del {
                        self.annotate.block_templates.del.enabled = true;
                        self.annotate.block_templates.del.start =
                            legacy_policy_to_start_template(&del);
                        self.annotate.block_templates.del.end = compatibility_end_template("del");
                    }
                }
            }
            if let Some(file_rules) = annotate.file_rules {
                self.annotate.file_rules = file_rules;
            }
            self.annotate.normalize();
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

    cfg.annotate.normalize();

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
    if let Some(value) = env_map.get("XGIT_FEATURE_RESET") {
        features.reset = Some(parse_bool_env("XGIT_FEATURE_RESET", value)?);
    }
    if let Some(value) = env_map.get("XGIT_FEATURE_CHECKOUT_REMOTE") {
        features.checkout_remote = Some(parse_bool_env("XGIT_FEATURE_CHECKOUT_REMOTE", value)?);
    }
    if let Some(value) = env_map.get("XGIT_FEATURE_COMPLETION") {
        features.completion = Some(parse_bool_env("XGIT_FEATURE_COMPLETION", value)?);
    }
    if features.push.is_some()
        || features.annotate.is_some()
        || features.reset.is_some()
        || features.checkout_remote.is_some()
        || features.completion.is_some()
    {
        partial.features = Some(features);
    }

    if let Some(value) = env_map.get("XGIT_ANNOTATE_INCLUDE_UNTRACKED") {
        partial.annotate = Some(PartialAnnotateConfig {
            staged: Some(PartialStagedConfig {
                include_untracked: Some(parse_bool_env("XGIT_ANNOTATE_INCLUDE_UNTRACKED", value)?),
            }),
            form: None,
            reference_kinds: None,
            date: None,
            render: None,
            old_code: None,
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

pub fn builtin_start_template(kind: &str) -> String {
    match kind {
        "add" => {
            "// {author_tag} {date} add: {reason} ({reference_kind}:{reference_value})".to_string()
        }
        "modify" => {
            "// {author_tag} {date} modify: {reason} ({reference_kind}:{reference_value}) old={old}"
                .to_string()
        }
        "del" => {
            "// {author_tag} {date} del: {reason} ({reference_kind}:{reference_value}) old={old}"
                .to_string()
        }
        _ => String::new(),
    }
}

pub fn default_block_template(kind: &str) -> BlockTemplate {
    BlockTemplate {
        enabled: false,
        start: builtin_start_template(kind),
        end: compatibility_end_template(kind),
    }
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
    use super::{
        merge_layers, project_config_path, resolve_git_root, AnnotateOldCodeLineLayout,
        AnnotateOldCodeMode,
    };
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
    fn default_date_and_old_code_are_initialized() {
        let env = HashMap::new();
        let cfg = merge_layers(None, None, &env, "zh-CN").unwrap();
        assert_eq!(cfg.annotate.date.format, "yyyy-mm-dd");
        assert!(cfg.annotate.old_code.enabled);
        assert_eq!(cfg.annotate.old_code.mode, None);
        assert_eq!(
            cfg.annotate.old_code.line_comment.layout,
            AnnotateOldCodeLineLayout::PerLine
        );
        assert_eq!(cfg.annotate.old_code.line_comment.body_prefix, "old: ");
        assert_eq!(cfg.annotate.old_code.block_comment.title, "cover old codes");
    }

    #[test]
    fn date_and_old_code_can_be_overridden_by_layers() {
        let project = r#"
[annotate.date]
format = "yyyy/mm/dd"

[annotate.old_code]
enabled = true
mode = "line_comment"

[annotate.old_code.line_comment]
layout = "header_body"
header = "legacy old"
body_prefix = "old=>"
body_suffix = ";"
"#;
        let env = HashMap::new();
        let cfg = merge_layers(None, Some(project), &env, "zh-CN").unwrap();
        assert_eq!(cfg.annotate.date.format, "yyyy/mm/dd");
        assert_eq!(
            cfg.annotate.old_code.mode,
            Some(AnnotateOldCodeMode::LineComment)
        );
        assert_eq!(
            cfg.annotate.old_code.line_comment.layout,
            AnnotateOldCodeLineLayout::HeaderBody
        );
        assert_eq!(cfg.annotate.old_code.line_comment.header, "legacy old");
        assert_eq!(cfg.annotate.old_code.line_comment.body_prefix, "old=>");
        assert_eq!(cfg.annotate.old_code.line_comment.body_suffix, ";");
    }

    #[test]
    fn legacy_old_code_none_mode_disables_without_overwriting_mode() {
        let project = r#"
[annotate.old_code]
mode = "none"
"#;
        let env = HashMap::new();
        let cfg = merge_layers(None, Some(project), &env, "zh-CN").unwrap();
        assert!(!cfg.annotate.old_code.enabled);
        assert_eq!(cfg.annotate.old_code.mode, None);
    }

    #[test]
    fn old_code_can_be_disabled_while_preserving_explicit_mode() {
        let project = r#"
[annotate.old_code]
enabled = false
mode = "block_comment"
"#;
        let env = HashMap::new();
        let cfg = merge_layers(None, Some(project), &env, "zh-CN").unwrap();
        assert!(!cfg.annotate.old_code.enabled);
        assert_eq!(
            cfg.annotate.old_code.mode,
            Some(AnnotateOldCodeMode::BlockComment)
        );
    }

    #[test]
    fn old_code_mode_survives_disable_save_and_reload() {
        let mut config = crate::config::AppConfig::default();
        config.annotate.old_code.enabled = false;
        config.annotate.old_code.mode = Some(AnnotateOldCodeMode::LineComment);

        let serialized = toml::to_string_pretty(&config).unwrap();
        let env = HashMap::new();
        let reloaded = merge_layers(None, Some(serialized.as_str()), &env, "zh-CN").unwrap();

        assert!(!reloaded.annotate.old_code.enabled);
        assert_eq!(
            reloaded.annotate.old_code.mode,
            Some(AnnotateOldCodeMode::LineComment)
        );
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

    #[test]
    fn disabled_template_roundtrip_preserves_custom_value() {
        let mut config = crate::config::AppConfig::default();
        config.annotate.block_templates.modify.enabled = false;
        config.annotate.block_templates.modify.start = "// custom modify value".to_string();
        config.annotate.block_templates.modify.end = "// custom modify end".to_string();

        let serialized = toml::to_string_pretty(&config).unwrap();
        let env = HashMap::new();
        let reloaded = merge_layers(None, Some(serialized.as_str()), &env, "zh-CN").unwrap();

        assert!(!reloaded.annotate.block_templates.modify.enabled);
        assert_eq!(
            reloaded.annotate.block_templates.modify.start,
            "// custom modify value"
        );
        assert_eq!(
            reloaded.annotate.block_templates.modify.end,
            "// custom modify end"
        );
    }
}
