use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use csslint_core::{Diagnostic, FileId};
use serde::Deserialize;

const TARGET_RULES: [&str; 3] = [
    "no_global_leaks",
    "no_duplicate_selectors",
    "no_overqualified_selectors",
];

#[derive(Debug, Deserialize)]
struct ExpectedRuleCase {
    rule_counts: BTreeMap<String, usize>,
}

#[test]
fn framework_rule_fixtures_match_expected_native_behavior() {
    for fixture_root in [
        native_fixture_root().join("vue/rules"),
        native_fixture_root().join("svelte/rules"),
    ] {
        let case_dirs = fixture_case_dirs(&fixture_root);
        assert!(
            !case_dirs.is_empty(),
            "expected rule fixture cases in {}",
            fixture_root.display()
        );

        for case_dir in case_dirs {
            let input_path = fixture_input_path(&case_dir);
            let source = fs::read_to_string(&input_path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
            let expected = read_json_fixture::<ExpectedRuleCase>(&case_dir.join("expected.json"));

            for key in expected.rule_counts.keys() {
                assert!(
                    TARGET_RULES.contains(&key.as_str()),
                    "unexpected rule id '{}' in {}",
                    key,
                    case_dir.display()
                );
            }

            let diagnostics = lint_source(&input_path, &source, FileId::new(500));
            let actual = collect_target_rule_counts(&diagnostics);
            let expected_counts = complete_expected_counts(expected.rule_counts);

            assert_eq!(
                actual,
                expected_counts,
                "target rule count mismatch for {}",
                case_dir.display()
            );
        }
    }
}

fn lint_source(path: &Path, source: &str, file_id: FileId) -> Vec<Diagnostic> {
    let extraction = csslint_extractor::extract_styles(file_id, path, source);
    assert!(
        extraction.diagnostics.is_empty(),
        "unexpected extraction diagnostics for {}",
        path.display()
    );

    let mut diagnostics = Vec::new();
    for style in extraction.styles {
        let parsed = csslint_parser::parse_style(&style)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error:?}", path.display()));
        let semantic = csslint_semantic::build_semantic_model(&parsed);
        diagnostics.extend(csslint_rules::run_rules(&semantic));
    }

    diagnostics
}

fn collect_target_rule_counts(diagnostics: &[Diagnostic]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for rule_id in TARGET_RULES {
        let count = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id.as_str() == rule_id)
            .count();
        counts.insert(rule_id.to_string(), count);
    }
    counts
}

fn complete_expected_counts(mut input: BTreeMap<String, usize>) -> BTreeMap<String, usize> {
    let mut expected = BTreeMap::new();
    for rule_id in TARGET_RULES {
        expected.insert(rule_id.to_string(), input.remove(rule_id).unwrap_or(0));
    }
    expected
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
