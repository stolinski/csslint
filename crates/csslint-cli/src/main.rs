#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use csslint_config::{format_diagnostics, load_for_target};
use csslint_core::{Diagnostic, FileId, LineIndex};
use csslint_extractor::extract_styles;
use csslint_fix::apply_fixes;
use csslint_parser::parse_style;
use csslint_rules::{run_rules_with_config, sort_diagnostics};
use csslint_semantic::build_semantic_model;
use serde::Serialize;

fn main() {
    let exit_code = match parse_cli_options(env::args()) {
        Ok(options) => match run_lint(&options) {
            Ok(result) => {
                print_result(&result, options.format);
                result.exit_code()
            }
            Err(error) => {
                eprintln!("{error}");
                2
            }
        },
        Err(error) => {
            eprintln!("{error}");
            2
        }
    };

    std::process::exit(exit_code);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    target_path: PathBuf,
    config_path: Option<PathBuf>,
    fix: bool,
    format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliErrorKind {
    Usage,
    Config,
    Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliError {
    kind: CliErrorKind,
    message: String,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: CliErrorKind::Usage,
            message: message.into(),
        }
    }

    fn config(message: impl Into<String>) -> Self {
        Self {
            kind: CliErrorKind::Config,
            message: message.into(),
        }
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self {
            kind: CliErrorKind::Runtime,
            message: message.into(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.kind {
            CliErrorKind::Usage => "usage_error",
            CliErrorKind::Config => "config_error",
            CliErrorKind::Runtime => "runtime_error",
        };
        write!(f, "csslint {label}: {}", self.message)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonResult {
    files_scanned: usize,
    files_linted: usize,
    fixes_applied: usize,
    diagnostics: Vec<RenderedDiagnostic>,
}

#[derive(Debug, Clone)]
struct LintResult {
    files_scanned: usize,
    files_linted: usize,
    fixes_applied: usize,
    diagnostics: Vec<RenderedDiagnostic>,
}

impl LintResult {
    fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "error")
            .count()
    }

    fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "warn")
            .count()
    }

    fn exit_code(&self) -> i32 {
        if self.error_count() > 0 {
            1
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderedDiagnostic {
    file_path: String,
    rule_id: String,
    severity: String,
    message: String,
    span: RenderedSpan,
    fix: RenderedFix,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderedSpan {
    start_offset: usize,
    end_offset: usize,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderedFix {
    available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacement: Option<String>,
}

fn parse_cli_options<I>(args: I) -> Result<CliOptions, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let _binary = args.next();

    let mut target_path: Option<PathBuf> = None;
    let mut config_path: Option<PathBuf> = None;
    let mut fix = false;
    let mut format = OutputFormat::Pretty;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fix" => {
                fix = true;
            }
            "--config" => {
                let Some(value) = args.next() else {
                    return Err(CliError::usage(
                        "missing value for --config (expected path to JSON .csslint file)",
                    ));
                };

                if config_path.is_some() {
                    return Err(CliError::usage("--config may only be provided once"));
                }
                config_path = Some(PathBuf::from(value));
            }
            "--format" => {
                let Some(value) = args.next() else {
                    return Err(CliError::usage(
                        "missing value for --format (expected 'json' or 'pretty')",
                    ));
                };

                format = match value.as_str() {
                    "json" => OutputFormat::Json,
                    "pretty" => OutputFormat::Pretty,
                    _ => {
                        return Err(CliError::usage(format!(
                            "unsupported format '{value}' (expected 'json' or 'pretty')"
                        )));
                    }
                };
            }
            "-h" | "--help" => {
                return Err(CliError::usage(
                    "usage: csslint <path> [--config <path>] [--fix] [--format json|pretty]",
                ));
            }
            _ if arg.starts_with('-') => {
                return Err(CliError::usage(format!("unknown flag '{arg}'")));
            }
            _ => {
                if target_path.is_some() {
                    return Err(CliError::usage(
                        "multiple input paths provided; only one path is supported in v1",
                    ));
                }
                target_path = Some(PathBuf::from(arg));
            }
        }
    }

    let Some(target_path) = target_path else {
        return Err(CliError::usage(
            "missing path argument; usage: csslint <path> [--config <path>] [--fix] [--format json|pretty]",
        ));
    };

    Ok(CliOptions {
        target_path,
        config_path,
        fix,
        format,
    })
}

fn run_lint(options: &CliOptions) -> Result<LintResult, CliError> {
    let loaded_config = load_for_target(&options.target_path, options.config_path.as_deref())
        .map_err(|diagnostics| CliError::config(format_diagnostics(&diagnostics)))?;
    let config = loaded_config.config;

    let target_files = discover_target_files(&options.target_path).map_err(|error| {
        CliError::runtime(format!(
            "failed to discover lint targets under '{}': {error}",
            options.target_path.display()
        ))
    })?;

    let files_scanned = target_files.len();
    let mut file_indexes: BTreeMap<FileId, (PathBuf, LineIndex)> = BTreeMap::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut fixes_applied = 0usize;

    for (index, file_path) in target_files.iter().enumerate() {
        let file_id = FileId::new(index as u32);
        let source = fs::read_to_string(file_path).map_err(|error| {
            CliError::runtime(format!("failed to read '{}': {error}", file_path.display()))
        })?;

        file_indexes.insert(file_id, (file_path.clone(), LineIndex::new(&source)));

        let extraction = extract_styles(file_id, file_path, &source);
        diagnostics.extend(extraction.diagnostics);

        for style in extraction.styles {
            match parse_style(&style) {
                Ok(parsed) => {
                    let semantic = build_semantic_model(&parsed);
                    match run_rules_with_config(&semantic, &config) {
                        Ok(rule_diagnostics) => diagnostics.extend(rule_diagnostics),
                        Err(config_diagnostics) => {
                            let message = config_diagnostics
                                .iter()
                                .map(|diagnostic| diagnostic.message.as_str())
                                .collect::<Vec<_>>()
                                .join("; ");
                            return Err(CliError::config(message));
                        }
                    }
                }
                Err(parse_diagnostic) => diagnostics.push(*parse_diagnostic),
            }
        }

        if options.fix {
            let file_fixes = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.file_id == file_id)
                .filter_map(|diagnostic| diagnostic.fix.clone())
                .collect::<Vec<_>>();

            let (updated, applied) = apply_fixes(&source, &file_fixes);
            if applied > 0 && updated != source {
                fs::write(file_path, updated).map_err(|error| {
                    CliError::runtime(format!(
                        "failed to write fixes to '{}': {error}",
                        file_path.display()
                    ))
                })?;
                fixes_applied += applied;
            }
        }
    }

    sort_diagnostics(&mut diagnostics);
    let rendered = diagnostics
        .into_iter()
        .filter_map(|diagnostic| render_diagnostic(&diagnostic, &file_indexes))
        .collect::<Vec<_>>();

    Ok(LintResult {
        files_scanned,
        files_linted: files_scanned,
        fixes_applied,
        diagnostics: rendered,
    })
}

fn render_diagnostic(
    diagnostic: &Diagnostic,
    file_indexes: &BTreeMap<FileId, (PathBuf, LineIndex)>,
) -> Option<RenderedDiagnostic> {
    let (file_path, line_index) = file_indexes.get(&diagnostic.file_id)?;
    let (start_line, start_column) = line_index.offset_to_line_column(diagnostic.span.start);
    let (end_line, end_column) = line_index.offset_to_line_column(diagnostic.span.end);

    let fix = match diagnostic.fix.as_ref() {
        Some(fix) => RenderedFix {
            available: true,
            start_offset: Some(fix.span.start),
            end_offset: Some(fix.span.end),
            replacement: Some(fix.replacement.clone()),
        },
        None => RenderedFix {
            available: false,
            start_offset: None,
            end_offset: None,
            replacement: None,
        },
    };

    Some(RenderedDiagnostic {
        file_path: file_path.to_string_lossy().to_string(),
        rule_id: diagnostic.rule_id.as_str().to_string(),
        severity: diagnostic.severity.as_str().to_string(),
        message: diagnostic.message.clone(),
        span: RenderedSpan {
            start_offset: diagnostic.span.start,
            end_offset: diagnostic.span.end,
            start_line,
            start_column,
            end_line,
            end_column,
        },
        fix,
    })
}

fn print_result(result: &LintResult, format: OutputFormat) {
    match format {
        OutputFormat::Pretty => print_pretty(result),
        OutputFormat::Json => print_json(result),
    }
}

fn print_pretty(result: &LintResult) {
    for diagnostic in &result.diagnostics {
        println!(
            "{}:{}:{} {} {} {}",
            diagnostic.file_path,
            diagnostic.span.start_line,
            diagnostic.span.start_column,
            diagnostic.severity,
            diagnostic.rule_id,
            diagnostic.message
        );
    }

    println!(
        "Scanned {} files, linted {}, errors: {}, warnings: {}, fixes applied: {}",
        result.files_scanned,
        result.files_linted,
        result.error_count(),
        result.warning_count(),
        result.fixes_applied,
    );
}

fn print_json(result: &LintResult) {
    let payload = JsonResult {
        files_scanned: result.files_scanned,
        files_linted: result.files_linted,
        fixes_applied: result.fixes_applied,
        diagnostics: result.diagnostics.clone(),
    };

    match serde_json::to_string_pretty(&payload) {
        Ok(json) => println!("{json}"),
        Err(error) => eprintln!("csslint runtime_error: failed to serialize json output: {error}"),
    }
}

fn discover_target_files(target: &Path) -> std::io::Result<Vec<PathBuf>> {
    if !target.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("target '{}' does not exist", target.display()),
        ));
    }

    if target.is_file() {
        if is_supported_lint_file(target) {
            return Ok(vec![target.to_path_buf()]);
        }
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    let mut pending = vec![target.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)?.collect::<Result<Vec<_>, std::io::Error>>()?;
        entries.sort_by(|left, right| {
            left.file_name()
                .to_string_lossy()
                .cmp(&right.file_name().to_string_lossy())
        });

        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if should_ignore_directory(&name) {
                    continue;
                }
                pending.push(path);
                continue;
            }

            if file_type.is_file() && is_supported_lint_file(&path) {
                files.push(path);
            }
        }
    }

    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    Ok(files)
}

fn is_supported_lint_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension, "css" | "vue" | "svelte"))
        .unwrap_or(false)
}

fn should_ignore_directory(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | "dist" | "build" | ".git" | ".hg" | ".svn"
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{discover_target_files, parse_cli_options, CliErrorKind, CliOptions, OutputFormat};

    #[test]
    fn parse_cli_options_accepts_v1_surface() {
        let parsed = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--fix".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(
            parsed,
            CliOptions {
                target_path: PathBuf::from("src"),
                config_path: None,
                fix: true,
                format: OutputFormat::Json,
            }
        );
    }

    #[test]
    fn parse_cli_options_requires_path_argument() {
        let error = parse_cli_options(["csslint".to_string()]).expect_err("path is required");
        assert_eq!(error.kind, CliErrorKind::Usage);
        assert!(error.message.contains("missing path argument"));
    }

    #[test]
    fn parse_cli_options_rejects_unknown_flags() {
        let error = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--wat".to_string(),
        ])
        .expect_err("unknown flag should fail");

        assert_eq!(error.kind, CliErrorKind::Usage);
        assert!(error.message.contains("unknown flag"));
    }

    #[test]
    fn parse_cli_options_accepts_explicit_config_path() {
        let parsed = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--config".to_string(),
            "configs/lint.json".to_string(),
        ])
        .expect("explicit config path should parse");

        assert_eq!(parsed.config_path, Some(PathBuf::from("configs/lint.json")));
    }

    #[test]
    fn parse_cli_options_requires_config_value() {
        let error = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--config".to_string(),
        ])
        .expect_err("--config without value should fail");

        assert_eq!(error.kind, CliErrorKind::Usage);
        assert!(error.message.contains("missing value for --config"));
    }

    #[test]
    fn discover_target_files_filters_supported_extensions_and_ignores_defaults() {
        let fixture = TempFixture::new("discovery-filter");

        fixture.write("src/main.css", "body { color: red; }");
        fixture.write("src/App.vue", "<style>.app { color: red; }</style>");
        fixture.write("src/Widget.svelte", "<style>.x { color: red; }</style>");
        fixture.write("src/ignore.js", "console.log('nope');");
        fixture.write("node_modules/pkg/index.css", "body {}");
        fixture.write("dist/generated.svelte", "<style>.x {}</style>");
        fixture.write("build/generated.vue", "<style>.x {}</style>");
        fixture.write(".git/config.css", "body {}");

        let files = discover_target_files(fixture.path()).expect("discovery should succeed");
        let relative = fixture.relative_paths(&files);
        assert_eq!(
            relative,
            vec![
                "src/App.vue".to_string(),
                "src/Widget.svelte".to_string(),
                "src/main.css".to_string(),
            ]
        );
    }

    #[test]
    fn discover_target_files_is_deterministic() {
        let fixture = TempFixture::new("discovery-deterministic");

        fixture.write("z.css", "z {}");
        fixture.write("a.vue", "<style>.a {}</style>");
        fixture.write("nested/b.svelte", "<style>.b {}</style>");

        let first = discover_target_files(fixture.path()).expect("first discovery should pass");
        let second = discover_target_files(fixture.path()).expect("second discovery should pass");

        assert_eq!(
            fixture.relative_paths(&first),
            fixture.relative_paths(&second)
        );
    }

    struct TempFixture {
        root: PathBuf,
    }

    impl TempFixture {
        fn new(label: &str) -> Self {
            let unique_suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos();

            let root = std::env::temp_dir().join(format!(
                "csslint-cli-{label}-{pid}-{unique_suffix}",
                pid = std::process::id()
            ));

            fs::create_dir_all(&root).expect("temp directory should be created");
            Self { root }
        }

        fn path(&self) -> &Path {
            &self.root
        }

        fn write(&self, relative_path: &str, contents: &str) {
            let full_path = self.root.join(relative_path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).expect("parent directory should be created");
            }
            fs::write(full_path, contents).expect("fixture file should be written");
        }

        fn relative_paths(&self, absolute_paths: &[PathBuf]) -> Vec<String> {
            absolute_paths
                .iter()
                .map(|path| {
                    path.strip_prefix(&self.root)
                        .expect("path should be inside fixture root")
                        .to_string_lossy()
                        .replace('\\', "/")
                })
                .collect()
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
