use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use csslint_core::{FileId, Scope};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ExpectedSelector {
    raw: String,
    normalized: String,
    part_scopes: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ExpectedSnapshot {
    selectors: Vec<ExpectedSelector>,
    declaration_props: Vec<String>,
    scope_index_counts: BTreeMap<String, usize>,
    class_index_keys: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct ActualSelector {
    raw: String,
    normalized: String,
    part_scopes: Vec<String>,
}

#[test]
fn semantic_snapshots_match_expected() {
    for case_dir in semantic_case_dirs() {
        let input_path = fixture_input_path(&case_dir);
        let source = fs::read_to_string(&input_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
        let expected_path = case_dir.join("expected.json");
        let expected: ExpectedSnapshot =
            serde_json::from_str(&fs::read_to_string(&expected_path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", expected_path.display())
            }))
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", expected_path.display()));

        let extraction = csslint_extractor::extract_styles(FileId::new(200), &input_path, &source);
        assert!(
            extraction.diagnostics.is_empty(),
            "unexpected extraction diagnostics for {}",
            case_dir.display()
        );
        assert_eq!(
            extraction.styles.len(),
            1,
            "semantic fixture should contain exactly one style block: {}",
            case_dir.display()
        );

        let parsed = csslint_parser::parse_style(&extraction.styles[0]).unwrap_or_else(|error| {
            panic!("failed to parse style in {}: {error:?}", case_dir.display())
        });
        let semantic = csslint_semantic::build_semantic_model(&parsed);

        let actual_selectors = semantic
            .selectors
            .iter()
            .map(|selector| ActualSelector {
                raw: selector.raw.clone(),
                normalized: selector.normalized.clone(),
                part_scopes: selector
                    .parts
                    .iter()
                    .map(|part| scope_to_string(part.scope).to_string())
                    .collect(),
            })
            .collect::<Vec<_>>();

        let expected_selectors = expected
            .selectors
            .iter()
            .map(|selector| ActualSelector {
                raw: selector.raw.clone(),
                normalized: selector.normalized.clone(),
                part_scopes: selector.part_scopes.clone(),
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual_selectors,
            expected_selectors,
            "selector snapshot mismatch for {}",
            case_dir.display()
        );

        let actual_declaration_props = semantic
            .declarations
            .iter()
            .map(|declaration| declaration.property.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            actual_declaration_props,
            expected.declaration_props,
            "declaration snapshot mismatch for {}",
            case_dir.display()
        );

        let mut actual_scope_counts = BTreeMap::new();
        for (scope, ids) in &semantic.indexes.selectors_by_scope {
            actual_scope_counts.insert(scope_to_string(*scope).to_string(), ids.len());
        }
        assert_eq!(
            actual_scope_counts,
            expected.scope_index_counts,
            "scope index mismatch for {}",
            case_dir.display()
        );

        let actual_class_index_keys = semantic
            .indexes
            .selectors_by_class
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            actual_class_index_keys,
            expected.class_index_keys,
            "class index keys mismatch for {}",
            case_dir.display()
        );
    }
}

fn scope_to_string(scope: Scope) -> &'static str {
    match scope {
        Scope::Global => "global",
        Scope::VueScoped => "vue_scoped",
        Scope::VueModule => "vue_module",
        Scope::SvelteScoped => "svelte_scoped",
    }
}

fn semantic_case_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/native/semantic");
    let mut entries = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read fixture root {}: {error}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

fn fixture_input_path(case_dir: &Path) -> PathBuf {
    let mut candidates = fs::read_dir(case_dir)
        .unwrap_or_else(|error| panic!("failed to read case {}: {error}", case_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("input."))
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("missing input file in fixture case {}", case_dir.display()))
}
