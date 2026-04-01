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

const C_CPP_ENTRIES: [CodeFileTypeEntry; 3] = [
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
];

const JAVA_ENTRIES: [CodeFileTypeEntry; 1] = [CodeFileTypeEntry {
    key: "java/java",
    label_key: "setup.code_file_type.entry.java.java",
    default_label: ".java",
    pattern: "*.java",
    renderer: C_LINE_BLOCK_RENDERER,
}];

const JAVASCRIPT_ENTRIES: [CodeFileTypeEntry; 1] = [CodeFileTypeEntry {
    key: "javascript/js",
    label_key: "setup.code_file_type.entry.javascript.js",
    default_label: ".js",
    pattern: "*.js",
    renderer: C_LINE_BLOCK_RENDERER,
}];

const RUST_ENTRIES: [CodeFileTypeEntry; 1] = [CodeFileTypeEntry {
    key: "rust/rs",
    label_key: "setup.code_file_type.entry.rust.rs",
    default_label: ".rs",
    pattern: "*.rs",
    renderer: C_LINE_BLOCK_RENDERER,
}];

const KOTLIN_ENTRIES: [CodeFileTypeEntry; 1] = [CodeFileTypeEntry {
    key: "kotlin/kt",
    label_key: "setup.code_file_type.entry.kotlin.kt",
    default_label: ".kt",
    pattern: "*.kt",
    renderer: C_LINE_BLOCK_RENDERER,
}];

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

fn builtin_entry_for_rule(rule: &FileRuleConfig) -> Option<&'static CodeFileTypeEntry> {
    categories()
        .iter()
        .flat_map(|category| category.entries.iter())
        .find(|entry| entry.pattern == rule.pattern && entry.renderer == rule.renderer)
}

#[cfg(test)]
mod tests {
    use super::{
        category_state, default_selected_keys, file_rules_from_selection,
        selection_from_file_rules, total_builtin_entry_count, TriState,
    };
    use crate::config::FileRuleConfig;
    use std::collections::BTreeSet;

    #[test]
    fn default_selection_enables_c_cpp_and_java() {
        let selected = default_selected_keys();
        assert!(selected.contains("c_cpp/c"));
        assert!(selected.contains("c_cpp/h"));
        assert!(selected.contains("c_cpp/cpp"));
        assert!(selected.contains("java/java"));
        assert_eq!(selected.len(), 4);
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
        assert_eq!(total_builtin_entry_count(), 7);
    }
}
