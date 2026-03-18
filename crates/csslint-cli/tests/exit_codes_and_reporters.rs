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

fn run_csslint(cwd: &Path, args: &[&str]) -> Output {
    Command::new(csslint_binary())
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("csslint command should execute")
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
