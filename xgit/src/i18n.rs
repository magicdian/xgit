use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const ZH_CN: &str = "zh-CN";
const EN_US: &str = "en-US";

#[derive(Debug, Clone)]
pub struct Catalog {
    messages: HashMap<String, String>,
}

impl Catalog {
    pub fn t(&self, key: &str) -> String {
        self.messages
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    pub fn tf(&self, key: &str, params: &[(&str, String)]) -> String {
        let mut rendered = self.t(key);
        for (name, value) in params {
            let placeholder = format!("{{{name}}}");
            rendered = rendered.replace(&placeholder, value);
        }
        rendered
    }
}

pub fn resolve_locale(env_lang: Option<String>, config_lang: Option<String>) -> String {
    for candidate in [env_lang, config_lang] {
        if let Some(locale) = normalize_locale(candidate.as_deref()) {
            return locale.to_string();
        }
    }
    detect_system_locale().to_string()
}

pub fn detect_system_locale() -> &'static str {
    let env_lang = std::env::var("LC_ALL")
        .ok()
        .or_else(|| std::env::var("LANG").ok());
    if let Some(raw) = env_lang {
        if let Some(locale) = normalize_locale(Some(raw.as_str())) {
            return if locale == ZH_CN { ZH_CN } else { EN_US };
        }
    }
    ZH_CN
}

pub fn normalize_locale(value: Option<&str>) -> Option<&'static str> {
    let lower = value?.trim().to_ascii_lowercase();
    if lower.starts_with("zh") {
        return Some(ZH_CN);
    }
    if lower.starts_with("en") {
        return Some(EN_US);
    }
    None
}

pub fn load_catalog(locale: &str, cwd: &Path) -> Result<Catalog> {
    let normalized = normalize_locale(Some(locale)).unwrap_or(ZH_CN);
    let raw = load_locale_source(normalized, cwd)?;
    let parsed: toml::Value = toml::from_str(&raw).context("failed to parse locale file")?;
    let message_table = parsed
        .get("messages")
        .and_then(toml::Value::as_table)
        .context("missing [messages] table in locale file")?;
    let mut messages = HashMap::new();
    flatten_messages("", message_table, &mut messages);
    Ok(Catalog { messages })
}

fn flatten_messages(
    prefix: &str,
    table: &toml::map::Map<String, toml::Value>,
    out: &mut HashMap<String, String>,
) {
    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            toml::Value::String(text) => {
                out.insert(full_key, text.to_string());
            }
            toml::Value::Table(nested) => flatten_messages(&full_key, nested, out),
            other => {
                out.insert(full_key, other.to_string());
            }
        }
    }
}

fn load_locale_source(locale: &str, cwd: &Path) -> Result<String> {
    for path in runtime_locale_candidates(locale, cwd) {
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read locale file: {}", path.display()))?;
            return Ok(content);
        }
    }

    let embedded = match locale {
        EN_US => include_str!("../resources/i18n/en-US.toml"),
        _ => include_str!("../resources/i18n/zh-CN.toml"),
    };
    Ok(embedded.to_string())
}

fn runtime_locale_candidates(locale: &str, cwd: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![cwd
        .join("resources")
        .join("i18n")
        .join(format!("{locale}.toml"))];

    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            candidates.push(
                bin_dir
                    .join("resources")
                    .join("i18n")
                    .join(format!("{locale}.toml")),
            );
            if let Some(parent) = bin_dir.parent() {
                candidates.push(
                    parent
                        .join("resources")
                        .join("i18n")
                        .join(format!("{locale}.toml")),
                );
            }
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::{normalize_locale, resolve_locale, EN_US, ZH_CN};

    #[test]
    fn normalize_variants() {
        assert_eq!(normalize_locale(Some("zh")), Some(ZH_CN));
        assert_eq!(normalize_locale(Some("zh_CN.UTF-8")), Some(ZH_CN));
        assert_eq!(normalize_locale(Some("en-US")), Some(EN_US));
    }

    #[test]
    fn locale_priority() {
        let locale = resolve_locale(Some("zh-CN".to_string()), Some("en-US".to_string()));
        assert_eq!(locale, ZH_CN);
    }
}
