#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

#[test]
fn exits_zero_when_no_error_diagnostics_are_reported() {
    let fixture = TempFixture::new("cli-exit-zero");
    fixture.write("main.css", ".app { color: red; }\n");

    let output = run_csslint(fixture.path(), &["main.css"]);
    assert_eq!(output.status.code(), Some(0));

    let stdout = normalize_line_endings(&output.stdout);
    assert!(stdout.contains("Summary:"));
    assert!(stdout.contains("0 error(s)"));
}

#[test]
fn long_version_flag_prints_version_and_exits_zero() {
    let fixture = TempFixture::new("cli-version-long");

    let output = run_csslint(fixture.path(), &["--version"]);
    assert_eq!(output.status.code(), Some(0));

    let stdout = normalize_line_endings(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("csslint {}", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn short_version_flag_prints_version_and_exits_zero() {
    let fixture = TempFixture::new("cli-version-short");

    let output = run_csslint(fixture.path(), &["-v"]);
    assert_eq!(output.status.code(), Some(0));

    let stdout = normalize_line_endings(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("csslint {}", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn exits_one_when_error_diagnostics_exist() {
    let fixture = TempFixture::new("cli-exit-one");
    fixture.write("main.css", ".app { colr: red; }\n");

    let output = run_csslint(fixture.path(), &["main.css"]);
    assert_eq!(output.status.code(), Some(1));

    let stdout = normalize_line_endings(&output.stdout);
    assert!(stdout.contains("no_unknown_properties"));
    assert!(stdout.contains("Summary:"));
}

#[test]
fn exits_two_on_config_failure() {
    let fixture = TempFixture::new("cli-exit-two-config");
    fixture.write("main.css", ".app { color: red; }\n");
    fixture.write(
        ".csslint",
        "{\n  \"rules\": {\n    \"no_not_real\": \"warn\"\n  }\n}\n",
    );

    let output = run_csslint(fixture.path(), &["main.css"]);
    assert_eq!(output.status.code(), Some(2));

    let stderr = normalize_line_endings(&output.stderr);
    assert!(stderr.contains("config_error"));
}

#[test]
fn exits_two_even_when_lint_errors_are_present_if_config_fails() {
    let fixture = TempFixture::new("cli-exit-two-precedence");
    fixture.write("main.css", ".app { colr: red; }\n");
    fixture.write(
        ".csslint",
        "{\n  \"rules\": {\n    \"no_not_real\": \"warn\"\n  }\n}\n",
    );

    let output = run_csslint(fixture.path(), &["main.css", "--format", "json"]);
    assert_eq!(output.status.code(), Some(2));

    let mut value: Value =
        serde_json::from_slice(&output.stdout).expect("json reporter should emit valid json");
    normalize_dynamic_timing_fields(&mut value);

    assert_eq!(value["summary"]["exitCode"], 2);
    assert_eq!(value["internalErrors"][0]["kind"], "config_error");
    assert_eq!(value["diagnostics"], Value::Array(Vec::new()));
}

#[test]
fn pretty_reporter_matches_snapshot() {
    let fixture = TempFixture::new("cli-pretty-snapshot");
    fixture.write("main.css", ".app { colr: red; }\n");

    let output = run_csslint(fixture.path(), &["main.css", "--code-frame"]);
    assert_eq!(output.status.code(), Some(1));

    let stdout = normalize_line_endings(&output.stdout);
    let expected = fs::read_to_string(snapshot_path("pretty-error.snap"))
        .expect("pretty snapshot should exist")
        .replace("\r\n", "\n");
    assert_eq!(stdout, expected);
}

#[test]
fn json_reporter_matches_snapshot_with_normalized_timing() {
    let fixture = TempFixture::new("cli-json-snapshot");
    fixture.write("main.css", ".app { colr: red; }\n");

    let output = run_csslint(fixture.path(), &["main.css", "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));

    let mut value: Value =
        serde_json::from_slice(&output.stdout).expect("json reporter should emit valid json");
    normalize_dynamic_timing_fields(&mut value);

    let expected: Value = serde_json::from_str(
        &fs::read_to_string(snapshot_path("json-error.snap.json"))
            .expect("json snapshot should exist"),
    )
    .expect("json snapshot should be valid json");

    assert_eq!(value, expected);
}

#[test]
fn json_reporter_always_includes_fix_object() {
    let fixture = TempFixture::new("cli-json-fix-contract");
    fixture.write(
        "main.css",
        ".app { colr: red; color: blue; color: blue; }\n",
    );

    let output = run_csslint(fixture.path(), &["main.css", "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));

    let value: Value =
        serde_json::from_slice(&output.stdout).expect("json reporter should emit valid json");
    let diagnostics = value["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");
    assert!(
        !diagnostics.is_empty(),
        "fixture should produce diagnostics for contract assertions"
    );

    for diagnostic in diagnostics {
        let fix = diagnostic
            .get("fix")
            .expect("diagnostic should include fix");
        assert!(fix["available"].is_boolean());
    }

    let unknown_property = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["ruleId"] == "no_unknown_properties")
        .expect("fixture should include unknown-property diagnostic");
    assert_eq!(unknown_property["fix"]["available"], Value::Bool(false));

    let duplicate_declaration = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["ruleId"] == "no_duplicate_declarations")
        .expect("fixture should include duplicate-declarations diagnostic");
    assert_eq!(duplicate_declaration["fix"]["available"], Value::Bool(true));
}

#[test]
fn rule_filter_limits_diagnostics_to_selected_rule_in_e2e() {
    let fixture = TempFixture::new("cli-rule-filter-e2e");
    fixture.write("main.css", ".dup, .dup { color: red; }\n.empty {}\n");

    let output = run_csslint(
        fixture.path(),
        &[
            "main.css",
            "--rule",
            "no_duplicate_selectors",
            "--format",
            "json",
        ],
    );
    assert_eq!(output.status.code(), Some(1));

    let value: Value =
        serde_json::from_slice(&output.stdout).expect("rule-filtered run should emit valid json");
    let diagnostics = value["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic["ruleId"] == "no_duplicate_selectors"));
}

#[test]
fn discovery_honors_csslintignore_and_ignore_path_override_in_e2e() {
    let fixture = TempFixture::new("cli-ignore-e2e");
    fixture.write(".csslintignore", "ignored-default.css\n");
    fixture.write("custom.ignore", "ignored-explicit.css\n");
    fixture.write("ignored-default.css", ".ignored-default { colr: red; }\n");
    fixture.write("ignored-explicit.css", ".ignored-explicit { colr: red; }\n");
    fixture.write("main.css", ".main { color: red; }\n");

    let default_output = run_csslint(fixture.path(), &[".", "--format", "json"]);
    assert_eq!(default_output.status.code(), Some(1));
    let default_json: Value =
        serde_json::from_slice(&default_output.stdout).expect("default run should emit valid json");

    let default_paths = diagnostic_paths(&default_json);
    assert!(
        default_paths
            .iter()
            .any(|path| path.ends_with("ignored-explicit.css")),
        "default discovery should lint files not listed in .csslintignore"
    );
    assert!(
        default_paths
            .iter()
            .all(|path| !path.ends_with("ignored-default.css")),
        "default discovery should skip .csslintignore matches"
    );

    let explicit_output = run_csslint(
        fixture.path(),
        &[".", "--ignore-path", "custom.ignore", "--format", "json"],
    );
    assert_eq!(explicit_output.status.code(), Some(1));
    let explicit_json: Value = serde_json::from_slice(&explicit_output.stdout)
        .expect("explicit ignore-path run should emit valid json");

    let explicit_paths = diagnostic_paths(&explicit_json);
    assert!(
        explicit_paths
            .iter()
            .any(|path| path.ends_with("ignored-default.css")),
        "explicit --ignore-path should replace default .csslintignore discovery"
    );
    assert!(
        explicit_paths
            .iter()
            .all(|path| !path.ends_with("ignored-explicit.css")),
        "explicit --ignore-path should filter configured matches"
    );
}

#[test]
fn fix_respects_stylelint_disable_and_ignore_path_combination_in_e2e() {
    let fixture = TempFixture::new("cli-fix-suppress-ignore-e2e");
    fixture.write(".csslintignore", "main.css\n");
    fixture.write("custom.ignore", "ignored-explicit.css\n");
    fixture.write(
        "main.css",
        "/* stylelint-disable-next-line declaration-block-no-duplicate-properties */\n.keep { color: red; color: red; }\n.fix { color: blue; color: blue; }\n",
    );
    fixture.write(
        "ignored-explicit.css",
        ".ignored { color: orange; color: orange; }\n",
    );

    let output = run_csslint(
        fixture.path(),
        &[
            ".",
            "--ignore-path",
            "custom.ignore",
            "--fix",
            "--format",
            "json",
        ],
    );
    assert_eq!(output.status.code(), Some(1));

    let value: Value =
        serde_json::from_slice(&output.stdout).expect("fix run should emit valid json");
    let diagnostics = value["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");

    let duplicate_main_count = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic["ruleId"] == "no_duplicate_declarations"
                && diagnostic
                    .get("filePath")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path.ends_with("main.css"))
        })
        .count();
    assert_eq!(
        duplicate_main_count, 1,
        "only unsuppressed duplicate in main.css should remain diagnostic-visible"
    );

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic
                .get("filePath")
                .and_then(Value::as_str)
                .map_or(true, |path| !path.ends_with("ignored-explicit.css"))
        }),
        "ignored-explicit.css should not produce diagnostics when excluded by --ignore-path"
    );

    let fixed_main = fs::read_to_string(fixture.path().join("main.css"))
        .expect("main.css should remain readable after fix");
    assert!(
        fixed_main.contains(".keep { color: red; color: red; }"),
        "suppressed duplicate declaration should remain untouched by --fix"
    );
    assert!(
        !fixed_main.contains(".fix { color: blue; color: blue; }"),
        "unsuppressed duplicate declaration should be fixed"
    );
    assert!(
        fixed_main.contains(".fix { color: blue; }"),
        "unsuppressed duplicate declaration should collapse to one declaration"
    );

    let ignored_source = fs::read_to_string(fixture.path().join("ignored-explicit.css"))
        .expect("ignored file should remain readable after fix");
    assert!(
        ignored_source.contains("color: orange; color: orange;"),
        "ignored file should be skipped entirely, including fix application"
    );
}

#[test]
fn stylelint_disable_enable_block_is_file_scoped_and_rule_scoped_in_e2e() {
    let fixture = TempFixture::new("cli-stylelint-disable-block-scope-e2e");
    fixture.write(
        "first.css",
        "/* stylelint-disable property-no-unknown */\n.one { colr: red; }\n.two { color: red; color: red; }\n/* stylelint-enable property-no-unknown */\n.three { colr: red; }\n",
    );
    fixture.write("second.css", ".alpha { colr: red; }\n");

    let output = run_csslint(fixture.path(), &[".", "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));

    let value: Value =
        serde_json::from_slice(&output.stdout).expect("report should emit valid json");
    let diagnostics = value["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");

    let unknown_first = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic["ruleId"] == "no_unknown_properties"
                && diagnostic
                    .get("filePath")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path.ends_with("first.css"))
        })
        .count();
    let unknown_second = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic["ruleId"] == "no_unknown_properties"
                && diagnostic
                    .get("filePath")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path.ends_with("second.css"))
        })
        .count();
    let duplicate_first = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic["ruleId"] == "no_duplicate_declarations"
                && diagnostic
                    .get("filePath")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path.ends_with("first.css"))
        })
        .count();

    assert_eq!(
        unknown_first, 1,
        "only declaration outside disable/enable block should report unknown property in first.css"
    );
    assert_eq!(
        unknown_second, 1,
        "stylelint-disable block in first.css must not leak suppression to second.css"
    );
    assert_eq!(
        duplicate_first, 1,
        "rule-scoped disable for property-no-unknown should not suppress duplicate declaration rule"
    );
}

#[test]
fn stylelint_disable_block_with_multiple_rules_controls_fixes_in_e2e() {
    let fixture = TempFixture::new("cli-stylelint-disable-multi-rule-fix-e2e");
    fixture.write(
        "main.css",
        "/* stylelint-disable declaration-block-no-duplicate-properties property-no-vendor-prefix */\n.keep { color: red; color: red; -webkit-transform: rotate(0); }\n/* stylelint-enable declaration-block-no-duplicate-properties property-no-vendor-prefix */\n.fix { color: blue; color: blue; -webkit-transform: rotate(0); }\n",
    );

    let output = run_csslint(fixture.path(), &["main.css", "--fix", "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));

    let value: Value =
        serde_json::from_slice(&output.stdout).expect("report should emit valid json");
    let diagnostics = value["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");

    let duplicate_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["ruleId"] == "no_duplicate_declarations")
        .count();
    let vendor_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["ruleId"] == "no_legacy_vendor_prefixes")
        .count();

    assert_eq!(
        duplicate_count, 1,
        "only unsuppressed duplicate declaration should remain diagnostic-visible"
    );
    assert_eq!(
        vendor_count, 1,
        "only unsuppressed vendor-prefixed property should remain diagnostic-visible"
    );

    let fixed_source =
        fs::read_to_string(fixture.path().join("main.css")).expect("main.css should be readable");
    assert!(
        fixed_source.contains(".keep { color: red; color: red; -webkit-transform: rotate(0); }"),
        "suppressed block should remain unchanged by --fix"
    );
    assert!(
        !fixed_source.contains(".fix { color: blue; color: blue; -webkit-transform: rotate(0); }"),
        "unsuppressed block should be transformed by --fix"
    );
    assert!(
        fixed_source.contains(".fix { color: blue; transform: rotate(0); }"),
        "unsuppressed block should receive both duplicate-removal and vendor-prefix fixes"
    );
}

#[test]
fn stylelint_disable_line_supports_comma_separated_rule_ids_in_e2e() {
    let fixture = TempFixture::new("cli-stylelint-disable-line-comma-e2e");
    fixture.write(
        "main.css",
        ".combo { /* stylelint-disable-line property-no-unknown, declaration-block-no-duplicate-properties */ colr: red; color: blue; color: blue; }\n.next { colr: red; color: green; color: green; }\n",
    );

    let output = run_csslint(fixture.path(), &["main.css", "--fix", "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));

    let value: Value =
        serde_json::from_slice(&output.stdout).expect("report should emit valid json");
    let diagnostics = value["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");

    let unknown_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["ruleId"] == "no_unknown_properties")
        .count();
    let duplicate_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["ruleId"] == "no_duplicate_declarations")
        .count();

    assert_eq!(
        unknown_count, 1,
        "comma-separated stylelint-disable-line should suppress unknown property only on targeted line"
    );
    assert_eq!(
        duplicate_count, 1,
        "comma-separated stylelint-disable-line should suppress duplicate declaration only on targeted line"
    );

    let fixed_source =
        fs::read_to_string(fixture.path().join("main.css")).expect("main.css should be readable");
    assert!(
        fixed_source.contains(".combo { /* stylelint-disable-line property-no-unknown, declaration-block-no-duplicate-properties */ colr: red; color: blue; color: blue; }"),
        "suppressed line should remain unchanged by --fix"
    );
    assert!(
        fixed_source.contains(".next { colr: red; color: green; }"),
        "unsuppressed line should be fixed"
    );
}

#[test]
fn nested_stylelint_disable_enable_ranges_are_respected_in_e2e() {
    let fixture = TempFixture::new("cli-stylelint-disable-nested-ranges-e2e");
    fixture.write(
        "main.css",
        "/* stylelint-disable property-no-unknown */\n.one { colr: red; }\n/* stylelint-disable property-no-unknown */\n.two { colr: red; }\n/* stylelint-enable property-no-unknown */\n.three { colr: red; }\n/* stylelint-enable property-no-unknown */\n.four { colr: red; }\n",
    );

    let output = run_csslint(fixture.path(), &["main.css", "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));

    let value: Value =
        serde_json::from_slice(&output.stdout).expect("report should emit valid json");
    let diagnostics = value["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");

    let unknown_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["ruleId"] == "no_unknown_properties")
        .count();
    assert_eq!(
        unknown_count, 1,
        "nested disable ranges should keep rule suppressed until final matching enable"
    );

    let only_message = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["ruleId"] == "no_unknown_properties")
        .and_then(|diagnostic| diagnostic.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        only_message.contains("colr"),
        "remaining unknown-property diagnostic should come from unsuppressed final block"
    );
}

#[test]
fn malformed_or_unterminated_stylelint_directives_do_not_crash_e2e() {
    let fixture = TempFixture::new("cli-stylelint-directive-resilience-e2e");
    fixture.write(
        "malformed.css",
        "/* stylelint-disablenextline property-no-unknown */\n.bad { colr: red; }\n",
    );
    fixture.write(
        "unterminated.css",
        "/* stylelint-disable-next-line property-no-unknown\n.bad { colr: red; }\n",
    );

    let malformed_output = run_csslint(fixture.path(), &["malformed.css", "--format", "json"]);
    assert_eq!(malformed_output.status.code(), Some(1));
    let malformed_value: Value =
        serde_json::from_slice(&malformed_output.stdout).expect("malformed run should emit json");
    let malformed_unknown_count = malformed_value["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array")
        .iter()
        .filter(|diagnostic| diagnostic["ruleId"] == "no_unknown_properties")
        .count();
    assert_eq!(
        malformed_unknown_count, 1,
        "unrecognized directive command should be ignored without suppressing diagnostics"
    );

    let unterminated_output =
        run_csslint(fixture.path(), &["unterminated.css", "--format", "json"]);
    assert_eq!(
        unterminated_output.status.code(),
        Some(0),
        "unterminated directive comment currently degrades to no-op lint input"
    );
    let unterminated_value: Value = serde_json::from_slice(&unterminated_output.stdout)
        .expect("unterminated directive run should still emit json");
    assert_eq!(
        unterminated_value["summary"]["exitCode"],
        Value::from(0),
        "json summary should align with process exit code"
    );
    assert_eq!(
        unterminated_value["diagnostics"],
        Value::Array(Vec::new()),
        "unterminated directive comment should not emit rule or parser diagnostics"
    );
    assert_eq!(
        unterminated_value["internalErrors"],
        Value::Array(Vec::new()),
        "unterminated directive comment should not trigger runtime/internal failures"
    );
}

fn run_csslint(cwd: &Path, args: &[&str]) -> Output {
    Command::new(csslint_binary())
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("csslint command should execute")
}

fn diagnostic_paths(report: &Value) -> Vec<String> {
    report["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array")
        .iter()
        .filter_map(|diagnostic| diagnostic.get("filePath").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

fn csslint_binary() -> &'static PathBuf {
    static CSSLINT_BINARY: OnceLock<PathBuf> = OnceLock::new();
    CSSLINT_BINARY.get_or_init(resolve_csslint_binary)
}

fn resolve_csslint_binary() -> PathBuf {
    if let Ok(binary) = std::env::var("CARGO_BIN_EXE_csslint") {
        return PathBuf::from(binary);
    }

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should resolve");
    let binary_name = if cfg!(windows) {
        "csslint.exe"
    } else {
        "csslint"
    };
    let fallback_binary = workspace_root
        .join("target")
        .join("debug")
        .join(binary_name);
    if fallback_binary.exists() {
        return fallback_binary;
    }

    let build_status = Command::new("cargo")
        .current_dir(&workspace_root)
        .args(["build", "-p", "csslint-cli", "--bin", "csslint"])
        .status()
        .expect("cargo build for csslint binary should run");
    assert!(
        build_status.success(),
        "cargo build for csslint binary should succeed"
    );

    fallback_binary
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(name)
}

fn normalize_dynamic_timing_fields(value: &mut Value) {
    value["summary"]["durationMs"] = Value::from(0);
    value["timing"]["parseMs"] = Value::from(0);
    value["timing"]["semanticMs"] = Value::from(0);
    value["timing"]["rulesMs"] = Value::from(0);
    value["timing"]["fixMs"] = Value::from(0);
}

fn normalize_line_endings(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

struct TempFixture {
    root: PathBuf,
}

impl TempFixture {
    fn new(label: &str) -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "csslint-cli-it-{label}-{pid}-{suffix}",
            pid = std::process::id()
        ));
        fs::create_dir_all(&root).expect("temp fixture directory should be created");
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn write(&self, relative: &str, contents: &str) {
        let full_path = self.root.join(relative);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("parent directory should exist");
        }
        fs::write(full_path, contents).expect("fixture file should be written");
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
