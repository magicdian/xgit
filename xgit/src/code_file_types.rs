use crate::config::FileRuleConfig;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriState {
    None,
    Partial,
    All,
}

#[derive(Debug, Clone, Copy)]
pub struct CodeFileTypeCategory {
    pub id: &'static str,
    pub label_key: &'static str,
    pub default_label: &'static str,
    pub entries: &'static [CodeFileTypeEntry],
}

#[derive(Debug, Clone, Copy)]
pub struct CodeFileTypeEntry {
    pub key: &'static str,
    pub label_key: &'static str,
    pub default_label: &'static str,
    pub pattern: &'static str,
    pub renderer: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeFileTypeSelection {
    pub selected_keys: BTreeSet<String>,
    pub passthrough_rules: Vec<FileRuleConfig>,
}

pub const C_LINE_BLOCK_RENDERER: &str = "c_line_block";

const C_CPP_ENTRIES: [CodeFileTypeEntry; 11] = [
    CodeFileTypeEntry {
        key: "c_cpp/c",
        label_key: "setup.code_file_type.entry.c_cpp.c",
        default_label: ".c",
        pattern: "*.c",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/h",
        label_key: "setup.code_file_type.entry.c_cpp.h",
        default_label: ".h",
        pattern: "*.h",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/cpp",
        label_key: "setup.code_file_type.entry.c_cpp.cpp",
        default_label: ".cpp",
        pattern: "*.cpp",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/cc",
        label_key: "setup.code_file_type.entry.c_cpp.cc",
        default_label: ".cc",
        pattern: "*.cc",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/cxx",
        label_key: "setup.code_file_type.entry.c_cpp.cxx",
        default_label: ".cxx",
        pattern: "*.cxx",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/hpp",
        label_key: "setup.code_file_type.entry.c_cpp.hpp",
        default_label: ".hpp",
        pattern: "*.hpp",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/hh",
        label_key: "setup.code_file_type.entry.c_cpp.hh",
        default_label: ".hh",
        pattern: "*.hh",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/hxx",
        label_key: "setup.code_file_type.entry.c_cpp.hxx",
        default_label: ".hxx",
        pattern: "*.hxx",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/ipp",
        label_key: "setup.code_file_type.entry.c_cpp.ipp",
        default_label: ".ipp",
        pattern: "*.ipp",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/inl",
        label_key: "setup.code_file_type.entry.c_cpp.inl",
        default_label: ".inl",
        pattern: "*.inl",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "c_cpp/tpp",
        label_key: "setup.code_file_type.entry.c_cpp.tpp",
        default_label: ".tpp",
        pattern: "*.tpp",
        renderer: C_LINE_BLOCK_RENDERER,
    },
];

const JAVA_ENTRIES: [CodeFileTypeEntry; 1] = [CodeFileTypeEntry {
    key: "java/java",
    label_key: "setup.code_file_type.entry.java.java",
    default_label: ".java",
    pattern: "*.java",
    renderer: C_LINE_BLOCK_RENDERER,
}];

const JAVASCRIPT_ENTRIES: [CodeFileTypeEntry; 4] = [
    CodeFileTypeEntry {
        key: "javascript/js",
        label_key: "setup.code_file_type.entry.javascript.js",
        default_label: ".js",
        pattern: "*.js",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "javascript/mjs",
        label_key: "setup.code_file_type.entry.javascript.mjs",
        default_label: ".mjs",
        pattern: "*.mjs",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "javascript/cjs",
        label_key: "setup.code_file_type.entry.javascript.cjs",
        default_label: ".cjs",
        pattern: "*.cjs",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "javascript/jsx",
        label_key: "setup.code_file_type.entry.javascript.jsx",
        default_label: ".jsx",
        pattern: "*.jsx",
        renderer: C_LINE_BLOCK_RENDERER,
    },
];

const RUST_ENTRIES: [CodeFileTypeEntry; 1] = [CodeFileTypeEntry {
    key: "rust/rs",
    label_key: "setup.code_file_type.entry.rust.rs",
    default_label: ".rs",
    pattern: "*.rs",
    renderer: C_LINE_BLOCK_RENDERER,
}];

const KOTLIN_ENTRIES: [CodeFileTypeEntry; 2] = [
    CodeFileTypeEntry {
        key: "kotlin/kt",
        label_key: "setup.code_file_type.entry.kotlin.kt",
        default_label: ".kt",
        pattern: "*.kt",
        renderer: C_LINE_BLOCK_RENDERER,
    },
    CodeFileTypeEntry {
        key: "kotlin/kts",
        label_key: "setup.code_file_type.entry.kotlin.kts",
        default_label: ".kts",
        pattern: "*.kts",
        renderer: C_LINE_BLOCK_RENDERER,
    },
];

const BUILTIN_CATEGORIES: [CodeFileTypeCategory; 5] = [
    CodeFileTypeCategory {
        id: "c_cpp",
        label_key: "setup.code_file_type.category.c_cpp",
        default_label: "C/C++",
        entries: &C_CPP_ENTRIES,
    },
    CodeFileTypeCategory {
        id: "java",
        label_key: "setup.code_file_type.category.java",
        default_label: "Java",
        entries: &JAVA_ENTRIES,
    },
    CodeFileTypeCategory {
        id: "javascript",
        label_key: "setup.code_file_type.category.javascript",
        default_label: "JavaScript",
        entries: &JAVASCRIPT_ENTRIES,
    },
    CodeFileTypeCategory {
        id: "rust",
        label_key: "setup.code_file_type.category.rust",
        default_label: "Rust",
        entries: &RUST_ENTRIES,
    },
    CodeFileTypeCategory {
        id: "kotlin",
        label_key: "setup.code_file_type.category.kotlin",
        default_label: "Kotlin",
        entries: &KOTLIN_ENTRIES,
    },
];

pub fn categories() -> &'static [CodeFileTypeCategory] {
    &BUILTIN_CATEGORIES
}

pub fn total_builtin_entry_count() -> usize {
    categories()
        .iter()
        .map(|category| category.entries.len())
        .sum()
}

pub fn default_selected_keys() -> BTreeSet<String> {
    categories()
        .iter()
        .filter(|category| category.id == "c_cpp" || category.id == "java")
        .flat_map(|category| category.entries.iter())
        .map(|entry| entry.key.to_string())
        .collect()
}

pub fn builtin_default_file_rules() -> Vec<FileRuleConfig> {
    let default_keys = default_selected_keys();
    file_rules_from_selection(&default_keys, &[])
}

pub fn selection_from_file_rules(rules: &[FileRuleConfig]) -> CodeFileTypeSelection {
    let mut selected_keys = BTreeSet::new();
    let mut passthrough_rules = Vec::new();

    for rule in rules {
        if let Some(entry) = builtin_entry_for_rule(rule) {
            selected_keys.insert(entry.key.to_string());
        } else {
            passthrough_rules.push(rule.clone());
        }
    }

    if rules.is_empty() {
        selected_keys = default_selected_keys();
    }

    CodeFileTypeSelection {
        selected_keys,
        passthrough_rules,
    }
}

pub fn file_rules_from_selection(
    selected_keys: &BTreeSet<String>,
    passthrough_rules: &[FileRuleConfig],
) -> Vec<FileRuleConfig> {
    let mut rules = Vec::new();

    for category in categories() {
        for entry in category.entries {
            if selected_keys.contains(entry.key) {
                rules.push(FileRuleConfig {
                    pattern: entry.pattern.to_string(),
                    renderer: entry.renderer.to_string(),
                });
            }
        }
    }

    for rule in passthrough_rules {
        if !rules.iter().any(|item| item == rule) {
            rules.push(rule.clone());
        }
    }

    rules
}

pub fn category_state(
    category: &CodeFileTypeCategory,
    selected_keys: &BTreeSet<String>,
) -> TriState {
    let selected = category_selected_count(category, selected_keys);
    if selected == 0 {
        TriState::None
    } else if selected == category.entries.len() {
        TriState::All
    } else {
        TriState::Partial
    }
}

pub fn category_selected_count(
    category: &CodeFileTypeCategory,
    selected_keys: &BTreeSet<String>,
) -> usize {
    category
        .entries
        .iter()
        .filter(|entry| selected_keys.contains(entry.key))
        .count()
}

pub fn set_category_selected(
    category: &CodeFileTypeCategory,
    selected_keys: &mut BTreeSet<String>,
    selected: bool,
) {
    for entry in category.entries {
        if selected {
            selected_keys.insert(entry.key.to_string());
        } else {
            selected_keys.remove(entry.key);
        }
    }
}

pub fn builtin_entry_for_path(path: &str) -> Option<&'static CodeFileTypeEntry> {
    categories()
        .iter()
        .flat_map(|category| category.entries.iter())
        .find(|entry| matches_path_pattern(path, entry.pattern))
}

fn builtin_entry_for_rule(rule: &FileRuleConfig) -> Option<&'static CodeFileTypeEntry> {
    categories()
        .iter()
        .flat_map(|category| category.entries.iter())
        .find(|entry| entry.pattern == rule.pattern && entry.renderer == rule.renderer)
}

fn matches_path_pattern(path: &str, pattern: &str) -> bool {
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
        builtin_entry_for_path, category_state, default_selected_keys, file_rules_from_selection,
        selection_from_file_rules, total_builtin_entry_count, TriState, C_LINE_BLOCK_RENDERER,
    };
    use crate::config::FileRuleConfig;
    use std::collections::BTreeSet;

    #[test]
    fn default_selection_enables_c_cpp_and_java() {
        let selected = default_selected_keys();
        assert!(selected.contains("c_cpp/c"));
        assert!(selected.contains("c_cpp/h"));
        assert!(selected.contains("c_cpp/cpp"));
        assert!(selected.contains("c_cpp/hpp"));
        assert!(selected.contains("c_cpp/tpp"));
        assert!(selected.contains("java/java"));
        assert_eq!(selected.len(), 12);
    }

    #[test]
    fn empty_rules_use_default_selection() {
        let selection = selection_from_file_rules(&[]);
        assert_eq!(selection.selected_keys, default_selected_keys());
        assert!(selection.passthrough_rules.is_empty());
    }

    #[test]
    fn roundtrip_preserves_custom_rules() {
        let rules = vec![
            FileRuleConfig {
                pattern: "*.c".to_string(),
                renderer: "c_line_block".to_string(),
            },
            FileRuleConfig {
                pattern: "*.proto".to_string(),
                renderer: "proto_line_block".to_string(),
            },
        ];
        let mut selection = selection_from_file_rules(&rules);
        selection.selected_keys.insert("javascript/js".to_string());

        let merged =
            file_rules_from_selection(&selection.selected_keys, &selection.passthrough_rules);
        assert!(merged
            .iter()
            .any(|rule| rule.pattern == "*.c" && rule.renderer == "c_line_block"));
        assert!(merged
            .iter()
            .any(|rule| rule.pattern == "*.js" && rule.renderer == "c_line_block"));
        assert!(merged
            .iter()
            .any(|rule| rule.pattern == "*.proto" && rule.renderer == "proto_line_block"));
    }

    #[test]
    fn category_reports_partial_state() {
        let categories = super::categories();
        let mut selected = BTreeSet::new();
        selected.insert("c_cpp/c".to_string());
        assert_eq!(category_state(&categories[0], &selected), TriState::Partial);
    }

    #[test]
    fn total_count_matches_catalog() {
        assert_eq!(total_builtin_entry_count(), 19);
    }

    #[test]
    fn builtin_default_rules_follow_default_selection_without_non_code_files() {
        let rules = super::builtin_default_file_rules();

        assert!(rules
            .iter()
            .any(|rule| rule.pattern == "*.hpp" && rule.renderer == C_LINE_BLOCK_RENDERER));
        assert!(rules
            .iter()
            .any(|rule| rule.pattern == "*.java" && rule.renderer == C_LINE_BLOCK_RENDERER));
        assert!(!rules.iter().any(|rule| rule.pattern == "Android.bp"));
        assert_eq!(rules.len(), 12);
    }

    #[test]
    fn builtin_entry_for_path_matches_known_builtin_suffix() {
        let entry = builtin_entry_for_path("networkmgr/routemgr/DnsEvent.cpp")
            .expect("cpp should be recognized as builtin type");
        assert_eq!(entry.pattern, "*.cpp");
    }

    #[test]
    fn builtin_entry_for_path_returns_none_for_unknown_type() {
        assert!(builtin_entry_for_path("build/Android.bp").is_none());
    }
}
