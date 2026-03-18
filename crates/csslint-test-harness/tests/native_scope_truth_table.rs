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

#[derive(Debug, Deserialize)]
struct ExpectedScopeCase {
    style_scope: String,
    selectors: Vec<ExpectedSelector>,
    scope_index_counts: BTreeMap<String, usize>,
}

#[derive(Debug, PartialEq, Eq)]
struct ActualSelector {
    raw: String,
    normalized: String,
    part_scopes: Vec<String>,
}

#[test]
fn css_defaults_to_global_scope() {
    let source = ".plain { color: red; }";
    let extraction =
        csslint_extractor::extract_styles(FileId::new(400), Path::new("fixture.css"), source);
    assert!(
        extraction.diagnostics.is_empty(),
        "plain CSS should not emit extraction diagnostics"
    );
    assert_eq!(
        extraction.styles.len(),
        1,
        "plain CSS should extract exactly one style block"
    );

    let style = &extraction.styles[0];
    assert_eq!(
        style.scope,
        Scope::Global,
        "plain CSS default scope should be global"
    );

    let parsed = csslint_parser::parse_style(style)
        .unwrap_or_else(|error| panic!("failed to parse CSS truth-table fixture: {error:?}"));
    let semantic = csslint_semantic::build_semantic_model(&parsed);
    assert_eq!(
        semantic.scope,
        Scope::Global,
        "semantic scope should remain global for plain CSS"
    );
    assert_eq!(semantic.selectors.len(), 1);

    let part_scopes = semantic.selectors[0]
        .parts
        .iter()
        .map(|part| part.scope)
        .collect::<Vec<_>>();
    assert_eq!(part_scopes, vec![Scope::Global]);
}

#[test]
fn native_scope_truth_table_fixtures_match_expected() {
    for fixture_root in [
        native_fixture_root().join("vue/scope"),
        native_fixture_root().join("svelte/scope"),
    ] {
        let case_dirs = fixture_case_dirs(&fixture_root);
        assert!(
            !case_dirs.is_empty(),
            "expected scope fixture cases in {}",
            fixture_root.display()
        );

        for case_dir in case_dirs {
            let input_path = fixture_input_path(&case_dir);
            let source = fs::read_to_string(&input_path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
            let expected = read_json_fixture::<ExpectedScopeCase>(&case_dir.join("expected.json"));

            let extraction =
                csslint_extractor::extract_styles(FileId::new(401), &input_path, &source);
            assert!(
                extraction.diagnostics.is_empty(),
                "unexpected extraction diagnostics for {}",
                case_dir.display()
            );
            assert_eq!(
                extraction.styles.len(),
                1,
                "scope fixture should contain exactly one style block: {}",
                case_dir.display()
            );

            let style = &extraction.styles[0];
            assert_eq!(
                scope_to_string(style.scope),
                expected.style_scope,
                "style scope mismatch for {}",
                case_dir.display()
            );

            let parsed = csslint_parser::parse_style(style).unwrap_or_else(|error| {
                panic!("failed to parse style in {}: {error:?}", case_dir.display())
            });
            let semantic = csslint_semantic::build_semantic_model(&parsed);

            assert_eq!(
                scope_to_string(semantic.scope),
                expected.style_scope,
                "semantic scope mismatch for {}",
                case_dir.display()
            );

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
                "selector scope snapshot mismatch for {}",
                case_dir.display()
            );

            let mut actual_scope_counts = BTreeMap::new();
            for (scope, ids) in &semantic.indexes.selectors_by_scope {
                actual_scope_counts.insert(scope_to_string(*scope).to_string(), ids.len());
            }
            assert_eq!(
                actual_scope_counts,
                expected.scope_index_counts,
                "scope index counts mismatch for {}",
                case_dir.display()
            );
        }
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

fn native_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/native")
}

fn fixture_case_dirs(root: &Path) -> Vec<PathBuf> {
    let mut entries = fs::read_dir(root)
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

fn read_json_fixture<T>(path: &Path) -> T
where
    T: for<'de> Deserialize<'de>,
{
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}
