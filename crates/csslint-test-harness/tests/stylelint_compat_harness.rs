use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use csslint_config::{canonical_rule_id_order, Config};
use csslint_core::{Diagnostic, FileId, LineIndex, RuleId, Severity, TargetProfile};
use csslint_fix::apply_fixes;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompatMode {
    Fast,
    Full,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureFile {
    stylelint: FixtureStylelint,
    csslint_rule: String,
    level: String,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureStylelint {
    rule: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureCase {
    id: String,
    kind: String,
    fast: bool,
    input: String,
    expected: ExpectedCase,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedCase {
    diagnostics: Vec<ExpectedDiagnostic>,
    fixed: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedDiagnostic {
    severity: String,
    message_contains: String,
    line: usize,
    column: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkipManifest {
    skips: Vec<SkipEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkipEntry {
    stylelint_rule: String,
    case_id: String,
}

#[derive(Debug, Default)]
struct HarnessOutcome {
    executed: usize,
    skipped: usize,
    passed: usize,
    failed: usize,
    failures: Vec<String>,
}

#[test]
fn compat_fast_suite_passes() {
    let outcome = run_harness(CompatMode::Fast);
    assert!(
        outcome.executed > 0,
        "compat-fast should execute at least one case"
    );
    assert_eq!(
        outcome.failed,
        0,
        "compat-fast failures:\n{}",
        outcome.failures.join("\n")
    );
}

#[test]
fn compat_full_suite_passes_with_manifest_skips() {
    let outcome = run_harness(CompatMode::Full);
    assert!(
        outcome.executed > 0,
        "compat-full should execute at least one case"
    );
    assert!(
        outcome.skipped > 0,
        "compat-full should include explicit manifest skips"
    );
    assert_eq!(
        outcome.failed,
        0,
        "compat-full failures:\n{}",
        outcome.failures.join("\n")
    );
}

fn run_harness(mode: CompatMode) -> HarnessOutcome {
    let fixtures = load_fixture_files();
    let skip_map = load_skip_manifest_map();
    let mut outcome = HarnessOutcome::default();

    for fixture in fixtures {
        let severity = match parse_severity(&fixture.level) {
            Ok(severity) => severity,
            Err(error) => {
                outcome.failed += 1;
                outcome.failures.push(format!(
                    "{}: invalid fixture level '{}': {error}",
                    fixture.stylelint.rule, fixture.level
                ));
                continue;
            }
        };

        for case in &fixture.cases {
            if mode == CompatMode::Fast && !case.fast {
                continue;
            }

            if skip_map.contains_key(&(fixture.stylelint.rule.clone(), case.id.clone())) {
                outcome.skipped += 1;
                continue;
            }

            outcome.executed += 1;
            match execute_case(&fixture, &case, severity, FileId::new(outcome.executed as u32 + 4000))
            {
                Ok(()) => outcome.passed += 1,
                Err(error) => {
                    outcome.failed += 1;
                    outcome.failures.push(format!(
                        "{}:{} ({}) -> {error}",
                        fixture.stylelint.rule, case.id, case.kind
                    ));
                }
            }
        }
    }

    outcome
}

fn execute_case(
    fixture: &FixtureFile,
    case: &FixtureCase,
    severity: Severity,
    file_id: FileId,
) -> Result<(), String> {
    let diagnostics = lint_case(
        &fixture.csslint_rule,
        severity,
        &case.input,
        file_id,
        &format!("{}.css", fixture.stylelint.rule),
    )?;

    if diagnostics.len() != case.expected.diagnostics.len() {
        return Err(format!(
            "diagnostic count mismatch (expected {}, got {})",
            case.expected.diagnostics.len(),
            diagnostics.len()
        ));
    }

    let line_index = LineIndex::new(&case.input);
    for (index, (actual, expected)) in diagnostics
        .iter()
        .zip(case.expected.diagnostics.iter())
        .enumerate()
    {
        if actual.severity.as_str() != expected.severity {
            return Err(format!(
                "diagnostic {index} severity mismatch (expected {}, got {})",
                expected.severity,
                actual.severity.as_str()
            ));
        }

        if !actual.message.contains(&expected.message_contains) {
            return Err(format!(
                "diagnostic {index} message mismatch (expected substring {:?}, got {:?})",
                expected.message_contains, actual.message
            ));
        }

        let (line, column) = line_index.offset_to_line_column(actual.span.start);
        if line != expected.line || column != expected.column {
            return Err(format!(
                "diagnostic {index} location mismatch (expected {}:{}, got {}:{})",
                expected.line, expected.column, line, column
            ));
        }
    }

    match &case.expected.fixed {
        Some(expected_fixed) => {
            let fixes = diagnostics
                .iter()
                .filter_map(|diagnostic| diagnostic.fix.clone())
                .collect::<Vec<_>>();
            if fixes.is_empty() {
                return Err("expected fix output but no fixes were reported".to_string());
            }

            let (updated, applied) = apply_fixes(&case.input, &fixes);
            if applied == 0 {
                return Err("expected at least one applied fix".to_string());
            }

            if &updated != expected_fixed {
                return Err(format!(
                    "fixed output mismatch (expected {:?}, got {:?})",
                    expected_fixed, updated
                ));
            }
        }
        None => {
            if diagnostics.iter().any(|diagnostic| diagnostic.fix.is_some()) {
                return Err("unexpected fix proposal for non-fix case".to_string());
            }
        }
    }

    Ok(())
}

fn lint_case(
    rule_id: &str,
    severity: Severity,
    source: &str,
    file_id: FileId,
    file_name: &str,
) -> Result<Vec<Diagnostic>, String> {
    let config = single_rule_config(rule_id, severity);
    let extraction = csslint_extractor::extract_styles(file_id, Path::new(file_name), source);

    let mut diagnostics = Vec::new();
    for extracted in extraction.styles {
        let parsed = csslint_parser::parse_style(&extracted)
            .map_err(|diagnostic| format!("parse failure: {}", diagnostic.message))?;
        let semantic = csslint_semantic::build_semantic_model(&parsed);
        let rule_diagnostics = csslint_rules::run_rules_with_config_and_targets(
            &semantic,
            &config,
            TargetProfile::Defaults,
        )
        .map_err(|config_diagnostics| {
            let messages = config_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            format!("config failure: {messages}")
        })?;

        diagnostics.extend(
            rule_diagnostics
                .into_iter()
                .filter(|diagnostic| diagnostic.rule_id.as_str() == rule_id),
        );
    }

    csslint_rules::sort_diagnostics(&mut diagnostics);
    Ok(diagnostics)
}

fn single_rule_config(rule_id: &str, severity: Severity) -> Config {
    let mut rules = BTreeMap::new();
    for known_rule in canonical_rule_id_order() {
        rules.insert(RuleId::from(*known_rule), Severity::Off);
    }
    rules.insert(RuleId::from(rule_id.to_string()), severity);

    Config { rules }
}

fn parse_severity(raw: &str) -> Result<Severity, String> {
    match raw {
        "off" => Ok(Severity::Off),
        "warn" => Ok(Severity::Warn),
        "error" => Ok(Severity::Error),
        _ => Err(format!("unknown level '{raw}'")),
    }
}

fn load_fixture_files() -> Vec<FixtureFile> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/compat/stylelint/imported");
    let mut fixture_paths = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read fixture root {}: {error}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<PathBuf>>();
    fixture_paths.sort();

    fixture_paths
        .into_iter()
        .map(|fixture_path| {
            let raw = fs::read_to_string(&fixture_path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", fixture_path.display()));
            serde_json::from_str::<FixtureFile>(&raw)
                .unwrap_or_else(|error| panic!("failed to parse {}: {error}", fixture_path.display()))
        })
        .collect()
}

fn load_skip_manifest_map() -> BTreeMap<(String, String), SkipEntry> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/compat/stylelint/skip-manifest.yaml");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let manifest: SkipManifest = serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));

    manifest
        .skips
        .into_iter()
        .map(|entry| ((entry.stylelint_rule.clone(), entry.case_id.clone()), entry))
        .collect()
}
