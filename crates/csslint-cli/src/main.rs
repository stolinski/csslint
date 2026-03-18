#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use csslint_config::{format_diagnostics, load_for_target, parse_target_profile};
use csslint_core::{Diagnostic, FileId, LineIndex};
use csslint_extractor::extract_styles;
use csslint_fix::apply_fixes;
use csslint_parser::{parse_style_with_options, CssParserOptions};
use csslint_rules::{run_rules_with_config_and_targets, sort_diagnostics};
use csslint_semantic::build_semantic_model;
use serde::Serialize;

fn main() {
    let exit_code = match parse_cli_options(env::args()) {
        Ok(options) => match run_lint(&options) {
            Ok(result) => {
                print_result(&result, options.format, options.code_frame);
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
    targets_override: Option<String>,
    code_frame: bool,
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
    file_sources: BTreeMap<String, String>,
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
    let mut targets_override: Option<String> = None;
    let mut code_frame = false;
    let mut fix = false;
    let mut format = OutputFormat::Pretty;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fix" => {
                fix = true;
            }
            "--code-frame" => {
                code_frame = true;
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
            "--targets" => {
                let Some(value) = args.next() else {
                    return Err(CliError::usage(
                        "missing value for --targets (expected target profile, e.g. defaults)",
                    ));
                };

                if targets_override.is_some() {
                    return Err(CliError::usage("--targets may only be provided once"));
                }
                targets_override = Some(value);
            }
            "-h" | "--help" => {
                return Err(CliError::usage(
                    "usage: csslint <path> [--config <path>] [--targets <profile>] [--code-frame] [--fix] [--format json|pretty]",
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
            "missing path argument; usage: csslint <path> [--config <path>] [--targets <profile>] [--code-frame] [--fix] [--format json|pretty]",
        ));
    };

    Ok(CliOptions {
        target_path,
        config_path,
        targets_override,
        code_frame,
        fix,
        format,
    })
}

fn run_lint(options: &CliOptions) -> Result<LintResult, CliError> {
    let loaded_config = load_for_target(&options.target_path, options.config_path.as_deref())
        .map_err(|diagnostics| CliError::config(format_diagnostics(&diagnostics)))?;
    let config = loaded_config.config;
    let target_profile = match options.targets_override.as_deref() {
        Some(raw_targets) => parse_target_profile(raw_targets).map_err(CliError::usage)?,
        None => loaded_config.targets,
    };

    let target_files = discover_target_files(&options.target_path).map_err(|error| {
        CliError::runtime(format!(
            "failed to discover lint targets under '{}': {error}",
            options.target_path.display()
        ))
    })?;

    let files_scanned = target_files.len();
    let mut file_indexes: BTreeMap<FileId, (PathBuf, LineIndex)> = BTreeMap::new();
    let mut file_sources: BTreeMap<String, String> = BTreeMap::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut fixes_applied = 0usize;

    for (index, file_path) in target_files.iter().enumerate() {
        let file_id = FileId::new(index as u32);
        let source = fs::read_to_string(file_path).map_err(|error| {
            CliError::runtime(format!("failed to read '{}': {error}", file_path.display()))
        })?;
        let display_path = file_path.to_string_lossy().to_string();
        file_sources.insert(display_path, source.clone());

        file_indexes.insert(file_id, (file_path.clone(), LineIndex::new(&source)));

        let extraction = extract_styles(file_id, file_path, &source);
        diagnostics.extend(extraction.diagnostics);

        for style in extraction.styles {
            match parse_style_with_options(
                &style,
                CssParserOptions {
                    enable_recovery: false,
                    targets: target_profile,
                },
            ) {
                Ok(parsed) => {
                    let semantic = build_semantic_model(&parsed);
                    match run_rules_with_config_and_targets(&semantic, &config, target_profile) {
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
        file_sources,
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

fn print_result(result: &LintResult, format: OutputFormat, code_frame: bool) {
    match format {
        OutputFormat::Pretty => print_pretty(result, code_frame),
        OutputFormat::Json => print_json(result),
    }
}

fn print_pretty(result: &LintResult, code_frame: bool) {
    let rendered = render_pretty(result, code_frame);
    print!("{rendered}");
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

fn render_pretty(result: &LintResult, code_frame: bool) -> String {
    let mut output = String::new();
    let mut current_file: Option<&str> = None;

    for diagnostic in &result.diagnostics {
        if current_file != Some(diagnostic.file_path.as_str()) {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&diagnostic.file_path);
            output.push('\n');
            current_file = Some(diagnostic.file_path.as_str());
        }

        output.push_str(&format!(
            "  {}:{}  {:<5}  {}  {}\n",
            diagnostic.span.start_line,
            diagnostic.span.start_column,
            diagnostic.severity,
            diagnostic.rule_id,
            diagnostic.message
        ));

        if code_frame {
            let frame = result
                .file_sources
                .get(&diagnostic.file_path)
                .and_then(|source| render_code_frame(source, diagnostic));
            if let Some(frame) = frame {
                for line in frame.lines() {
                    output.push_str("    ");
                    output.push_str(line);
                    output.push('\n');
                }
            }
        }
    }

    if !result.diagnostics.is_empty() {
        output.push('\n');
    }

    output.push_str(&format!(
        "Summary: {} error(s), {} warning(s), {} file(s) scanned, {} file(s) linted, {} fix(es) applied\n",
        result.error_count(),
        result.warning_count(),
        result.files_scanned,
        result.files_linted,
        result.fixes_applied
    ));

    output
}

fn render_code_frame(source: &str, diagnostic: &RenderedDiagnostic) -> Option<String> {
    let line_number = diagnostic.span.start_line;
    let line_text = source
        .lines()
        .nth(line_number.saturating_sub(1))?
        .trim_end_matches('\r');
    let gutter_width = line_number.to_string().len();
    let marker_offset = diagnostic.span.start_column.saturating_sub(1);
    let marker_length = if diagnostic.span.start_line == diagnostic.span.end_line {
        diagnostic
            .span
            .end_column
            .saturating_sub(diagnostic.span.start_column)
            .max(1)
    } else {
        1
    };

    let marker_line = format!(
        "{blank:>width$} | {offset}{marker}",
        blank = "",
        width = gutter_width,
        offset = " ".repeat(marker_offset),
        marker = "^".repeat(marker_length)
    );

    Some(format!(
        "{line_number:>width$} | {line_text}\n{marker_line}",
        width = gutter_width,
    ))
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
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        discover_target_files, parse_cli_options, render_pretty, CliErrorKind, CliOptions,
        LintResult, OutputFormat, RenderedDiagnostic, RenderedFix, RenderedSpan,
    };

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
                targets_override: None,
                code_frame: false,
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
        assert!(!parsed.code_frame);
    }

    #[test]
    fn parse_cli_options_accepts_targets_override() {
        let parsed = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--targets".to_string(),
            "defaults".to_string(),
        ])
        .expect("targets override should parse");

        assert_eq!(parsed.targets_override.as_deref(), Some("defaults"));
    }

    #[test]
    fn parse_cli_options_accepts_code_frame_flag() {
        let parsed = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--code-frame".to_string(),
        ])
        .expect("code frame flag should parse");

        assert!(parsed.code_frame);
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
    fn parse_cli_options_requires_targets_value() {
        let error = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--targets".to_string(),
        ])
        .expect_err("--targets without value should fail");

        assert_eq!(error.kind, CliErrorKind::Usage);
        assert!(error.message.contains("missing value for --targets"));
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

    #[test]
    fn pretty_reporter_is_deterministic_with_code_frames() {
        let mut file_sources = BTreeMap::new();
        file_sources.insert(
            "src/main.css".to_string(),
            ".alpha { color: red; }\n.beta { color: blue; }\n".to_string(),
        );

        let result = LintResult {
            files_scanned: 1,
            files_linted: 1,
            fixes_applied: 0,
            diagnostics: vec![RenderedDiagnostic {
                file_path: "src/main.css".to_string(),
                rule_id: "no_empty_rules".to_string(),
                severity: "warn".to_string(),
                message: "Example diagnostic".to_string(),
                span: RenderedSpan {
                    start_offset: 1,
                    end_offset: 6,
                    start_line: 1,
                    start_column: 2,
                    end_line: 1,
                    end_column: 7,
                },
                fix: RenderedFix {
                    available: false,
                    start_offset: None,
                    end_offset: None,
                    replacement: None,
                },
            }],
            file_sources,
        };

        let first = render_pretty(&result, true);
        let second = render_pretty(&result, true);
        let without_frame = render_pretty(&result, false);

        assert_eq!(first, second);
        assert!(first.contains("Summary:"));
        assert!(first.contains("^"));
        assert!(!without_frame.contains("^"));
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
