#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::Instant;

use csslint_config::{format_diagnostics, load_for_target, parse_target_profile};
use csslint_core::{Diagnostic, FileId, LineIndex};
use csslint_extractor::extract_styles;
use csslint_fix::apply_fixes;
use csslint_parser::{parse_style_with_options, CssParserOptions};
use csslint_rules::{run_rules_profiled_with_config_and_targets, sort_diagnostics};
use csslint_semantic::build_semantic_model;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::Serialize;

fn main() {
    let exit_code = match parse_cli_options(env::args()) {
        Ok(options) => match run_lint(&options) {
            Ok(result) => {
                print_result(&result, options.format, options.code_frame, options.profile);
                result.exit_code()
            }
            Err(error) => {
                if options.format == OutputFormat::Json {
                    print_json_error(&error);
                } else {
                    eprintln!("{error}");
                }
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
    ignore_path: Option<PathBuf>,
    targets_override: Option<String>,
    code_frame: bool,
    profile: bool,
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
    schema_version: u8,
    tool: &'static str,
    summary: JsonSummary,
    diagnostics: Vec<RenderedDiagnostic>,
    internal_errors: Vec<JsonInternalError>,
    timing: JsonTiming,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonSummary {
    files_scanned: usize,
    files_linted: usize,
    errors: usize,
    warnings: usize,
    fixes_applied: usize,
    duration_ms: usize,
    exit_code: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonInternalError {
    kind: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonTiming {
    parse_ms: usize,
    semantic_ms: usize,
    rules_ms: usize,
    fix_ms: usize,
}

#[derive(Debug, Clone)]
struct LintResult {
    files_scanned: usize,
    files_linted: usize,
    fixes_applied: usize,
    diagnostics: Vec<RenderedDiagnostic>,
    file_sources: BTreeMap<String, String>,
    internal_errors: Vec<InternalError>,
    timing: PhaseTiming,
    duration_ms: usize,
    file_profile: Vec<FileProfile>,
    rule_profile: BTreeMap<String, RuleProfileEntry>,
}

#[derive(Debug, Clone)]
struct FileProfile {
    path: String,
    total_ms: f64,
}

#[derive(Debug, Clone, Default)]
struct RuleProfileEntry {
    elapsed_ms: f64,
    diagnostics_emitted: usize,
}

#[derive(Debug, Clone)]
struct InternalError {
    kind: String,
    message: String,
    file_path: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct PhaseTiming {
    parse_ms: usize,
    semantic_ms: usize,
    rules_ms: usize,
    fix_ms: usize,
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
        if !self.internal_errors.is_empty() {
            return 2;
        }

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
    let mut ignore_path: Option<PathBuf> = None;
    let mut targets_override: Option<String> = None;
    let mut code_frame = false;
    let mut profile = false;
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
            "--profile" => {
                profile = true;
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
            "--ignore-path" => {
                let Some(value) = args.next() else {
                    return Err(CliError::usage(
                        "missing value for --ignore-path (expected path to .csslintignore file)",
                    ));
                };

                if ignore_path.is_some() {
                    return Err(CliError::usage("--ignore-path may only be provided once"));
                }
                ignore_path = Some(PathBuf::from(value));
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
                    "usage: csslint <path> [--config <path>] [--ignore-path <path>] [--targets <profile>] [--code-frame] [--profile] [--fix] [--format json|pretty]",
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
            "missing path argument; usage: csslint <path> [--config <path>] [--ignore-path <path>] [--targets <profile>] [--code-frame] [--profile] [--fix] [--format json|pretty]",
        ));
    };

    Ok(CliOptions {
        target_path,
        config_path,
        ignore_path,
        targets_override,
        code_frame,
        profile,
        fix,
        format,
    })
}

fn run_lint(options: &CliOptions) -> Result<LintResult, CliError> {
    let run_started_at = Instant::now();
    let loaded_config = load_for_target(&options.target_path, options.config_path.as_deref())
        .map_err(|diagnostics| CliError::config(format_diagnostics(&diagnostics)))?;
    let config = loaded_config.config;
    let target_profile = match options.targets_override.as_deref() {
        Some(raw_targets) => parse_target_profile(raw_targets).map_err(CliError::usage)?,
        None => loaded_config.targets,
    };

    let target_files = discover_target_files(&options.target_path, options.ignore_path.as_deref())
        .map_err(|error| {
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
    let mut timing = PhaseTiming::default();
    let mut file_profile = Vec::new();
    let mut rule_profile: BTreeMap<String, RuleProfileEntry> = BTreeMap::new();

    for (index, file_path) in target_files.iter().enumerate() {
        let file_started_at = Instant::now();
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
            let parse_started_at = Instant::now();
            let parsed_style = parse_style_with_options(
                &style,
                CssParserOptions {
                    enable_recovery: false,
                    targets: target_profile,
                },
            );
            timing.parse_ms += parse_started_at.elapsed().as_millis() as usize;

            match parsed_style {
                Ok(parsed) => {
                    let semantic_started_at = Instant::now();
                    let semantic = build_semantic_model(&parsed);
                    timing.semantic_ms += semantic_started_at.elapsed().as_millis() as usize;

                    let rules_started_at = Instant::now();
                    let rule_results = run_rules_profiled_with_config_and_targets(
                        &semantic,
                        &config,
                        target_profile,
                    );
                    timing.rules_ms += rules_started_at.elapsed().as_millis() as usize;

                    match rule_results {
                        Ok(rule_output) => {
                            diagnostics.extend(rule_output.diagnostics);
                            for profile_stat in rule_output.profile {
                                let entry = rule_profile
                                    .entry(profile_stat.rule_id.as_str().to_string())
                                    .or_default();
                                entry.elapsed_ms += profile_stat.elapsed_ms;
                                entry.diagnostics_emitted += profile_stat.diagnostics_emitted;
                            }
                        }
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

        if let Some((_, line_index)) = file_indexes.get(&file_id) {
            let suppression_plan = build_inline_suppression_plan(&source);
            diagnostics.retain(|diagnostic| {
                diagnostic.file_id != file_id
                    || !is_diagnostic_suppressed(diagnostic, line_index, &suppression_plan)
            });
        }

        if options.fix {
            let fix_started_at = Instant::now();
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
            timing.fix_ms += fix_started_at.elapsed().as_millis() as usize;
        }

        file_profile.push(FileProfile {
            path: file_path.to_string_lossy().to_string(),
            total_ms: file_started_at.elapsed().as_secs_f64() * 1000.0,
        });
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
        internal_errors: Vec::new(),
        timing,
        duration_ms: run_started_at.elapsed().as_millis() as usize,
        file_profile,
        rule_profile,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuppressionCommand {
    Disable,
    Enable,
    DisableLine,
    DisableNextLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SuppressionDirective {
    command: SuppressionCommand,
    rule_ids: Vec<String>,
    start_offset: usize,
    end_offset: usize,
    line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OffsetRange {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SuppressionPlan {
    all_ranges: Vec<OffsetRange>,
    rule_ranges: BTreeMap<String, Vec<OffsetRange>>,
    line_all: BTreeSet<usize>,
    line_by_rule: BTreeMap<String, BTreeSet<usize>>,
}

fn build_inline_suppression_plan(source: &str) -> SuppressionPlan {
    let directives = collect_suppression_directives(source);
    let mut plan = SuppressionPlan::default();

    let mut disabled_all_depth = 0usize;
    let mut disabled_all_start: Option<usize> = None;
    let mut disabled_rule_states: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for directive in directives {
        match directive.command {
            SuppressionCommand::Disable => {
                if directive.rule_ids.is_empty() {
                    if disabled_all_depth == 0 {
                        disabled_all_start = Some(directive.end_offset);
                    }
                    disabled_all_depth = disabled_all_depth.saturating_add(1);
                    continue;
                }

                for rule_id in directive.rule_ids {
                    disabled_rule_states
                        .entry(rule_id)
                        .and_modify(|(depth, _)| *depth = depth.saturating_add(1))
                        .or_insert((1, directive.end_offset));
                }
            }
            SuppressionCommand::Enable => {
                if directive.rule_ids.is_empty() {
                    if disabled_all_depth > 0 {
                        disabled_all_depth -= 1;
                        if disabled_all_depth == 0 {
                            if let Some(start) = disabled_all_start.take() {
                                plan.all_ranges.push(OffsetRange {
                                    start,
                                    end: directive.start_offset,
                                });
                            }
                        }
                    }

                    for (rule_id, (_, start)) in std::mem::take(&mut disabled_rule_states) {
                        plan.rule_ranges
                            .entry(rule_id)
                            .or_default()
                            .push(OffsetRange {
                                start,
                                end: directive.start_offset,
                            });
                    }
                    continue;
                }

                for rule_id in directive.rule_ids {
                    if let Some((depth, start)) = disabled_rule_states.get_mut(&rule_id) {
                        if *depth > 1 {
                            *depth -= 1;
                            continue;
                        }

                        let start_offset = *start;
                        disabled_rule_states.remove(&rule_id);
                        plan.rule_ranges
                            .entry(rule_id)
                            .or_default()
                            .push(OffsetRange {
                                start: start_offset,
                                end: directive.start_offset,
                            });
                    }
                }
            }
            SuppressionCommand::DisableLine => {
                register_line_suppression(&mut plan, directive.line, &directive.rule_ids);
            }
            SuppressionCommand::DisableNextLine => {
                register_line_suppression(
                    &mut plan,
                    directive.line.saturating_add(1),
                    &directive.rule_ids,
                );
            }
        }
    }

    let source_end = source.len();
    if let Some(start) = disabled_all_start {
        plan.all_ranges.push(OffsetRange {
            start,
            end: source_end,
        });
    }
    for (rule_id, (_, start)) in disabled_rule_states {
        plan.rule_ranges
            .entry(rule_id)
            .or_default()
            .push(OffsetRange {
                start,
                end: source_end,
            });
    }

    plan
}

fn register_line_suppression(plan: &mut SuppressionPlan, line: usize, rule_ids: &[String]) {
    if line == 0 {
        return;
    }

    if rule_ids.is_empty() {
        plan.line_all.insert(line);
        return;
    }

    for rule_id in rule_ids {
        plan.line_by_rule
            .entry(rule_id.clone())
            .or_default()
            .insert(line);
    }
}

fn collect_suppression_directives(source: &str) -> Vec<SuppressionDirective> {
    let mut directives = Vec::new();
    let line_index = LineIndex::new(source);
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while cursor + 1 < bytes.len() {
        if bytes[cursor] != b'/' || bytes[cursor + 1] != b'*' {
            cursor += 1;
            continue;
        }

        let comment_start = cursor;
        cursor += 2;
        while cursor + 1 < bytes.len() && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/') {
            cursor += 1;
        }

        if cursor + 1 >= bytes.len() {
            break;
        }

        let comment_end = cursor + 2;
        let comment_body = source.get(comment_start + 2..cursor).unwrap_or("");
        if let Some((command, rule_ids)) = parse_suppression_directive(comment_body) {
            let (line, _) = line_index.offset_to_line_column(comment_start);
            directives.push(SuppressionDirective {
                command,
                rule_ids,
                start_offset: comment_start,
                end_offset: comment_end,
                line,
            });
        }

        cursor = comment_end;
    }

    directives
}

fn parse_suppression_directive(comment_body: &str) -> Option<(SuppressionCommand, Vec<String>)> {
    let normalized = normalize_comment_body(comment_body);
    if normalized.is_empty() {
        return None;
    }

    let mut parts = normalized.splitn(2, char::is_whitespace);
    let command = parts.next()?.to_ascii_lowercase();
    let rest = parts
        .next()
        .unwrap_or("")
        .split("--")
        .next()
        .unwrap_or("")
        .trim();

    let suppression_command = match command.as_str() {
        "csslint-disable" | "stylelint-disable" => SuppressionCommand::Disable,
        "csslint-enable" | "stylelint-enable" => SuppressionCommand::Enable,
        "csslint-disable-line" | "stylelint-disable-line" => SuppressionCommand::DisableLine,
        "csslint-disable-next-line" | "stylelint-disable-next-line" => {
            SuppressionCommand::DisableNextLine
        }
        _ => return None,
    };

    Some((suppression_command, parse_rule_ids(rest)))
}

fn normalize_comment_body(comment_body: &str) -> String {
    comment_body
        .lines()
        .map(|line| line.trim())
        .map(|line| line.strip_prefix('*').unwrap_or(line).trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_rule_ids(raw_rules: &str) -> Vec<String> {
    if raw_rules.trim().is_empty() {
        return Vec::new();
    }

    let mut rule_ids = raw_rules
        .replace(',', " ")
        .split_whitespace()
        .map(canonicalize_suppression_rule_id)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if rule_ids.iter().any(|rule_id| rule_id == "all") {
        return Vec::new();
    }

    rule_ids.sort();
    rule_ids.dedup();
    rule_ids
}

fn canonicalize_suppression_rule_id(token: &str) -> String {
    let canonical = token.trim().to_ascii_lowercase().replace('-', "_");
    match canonical.as_str() {
        "block_no_empty" => "no_empty_rules".to_string(),
        "declaration_block_no_duplicate_properties" => "no_duplicate_declarations".to_string(),
        "declaration_property_value_no_unknown" => "no_invalid_values".to_string(),
        "property_no_unknown" => "no_unknown_properties".to_string(),
        "property_no_vendor_prefix" | "value_no_vendor_prefix" => {
            "no_legacy_vendor_prefixes".to_string()
        }
        "selector_no_qualifying_type" => "no_overqualified_selectors".to_string(),
        _ => canonical,
    }
}

fn is_diagnostic_suppressed(
    diagnostic: &Diagnostic,
    line_index: &LineIndex,
    plan: &SuppressionPlan,
) -> bool {
    let offset = diagnostic.span.start;
    let (line, _) = line_index.offset_to_line_column(offset);
    let rule_id = diagnostic.rule_id.as_str().to_ascii_lowercase();

    if plan.line_all.contains(&line) {
        return true;
    }
    if plan
        .line_by_rule
        .get(&rule_id)
        .is_some_and(|lines| lines.contains(&line))
    {
        return true;
    }

    if offset_in_ranges(offset, &plan.all_ranges) {
        return true;
    }
    plan.rule_ranges
        .get(&rule_id)
        .is_some_and(|ranges| offset_in_ranges(offset, ranges))
}

fn offset_in_ranges(offset: usize, ranges: &[OffsetRange]) -> bool {
    ranges
        .iter()
        .any(|range| offset >= range.start && offset < range.end)
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

fn print_result(result: &LintResult, format: OutputFormat, code_frame: bool, profile: bool) {
    match format {
        OutputFormat::Pretty => print_pretty(result, code_frame),
        OutputFormat::Json => print_json(result),
    }

    if profile {
        print_profile_report(result);
    }
}

fn print_pretty(result: &LintResult, code_frame: bool) {
    let rendered = if should_use_color_output() {
        render_pretty_colored(result, code_frame)
    } else {
        render_pretty(result, code_frame)
    };
    print!("{rendered}");
}

fn should_use_color_output() -> bool {
    std::io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none()
}

fn print_json(result: &LintResult) {
    match render_json(result) {
        Ok(json) => println!("{json}"),
        Err(error) => eprintln!("csslint runtime_error: failed to serialize json output: {error}"),
    }
}

fn print_json_error(error: &CliError) {
    let payload = JsonResult {
        schema_version: 1,
        tool: "csslint",
        summary: JsonSummary {
            files_scanned: 0,
            files_linted: 0,
            errors: 0,
            warnings: 0,
            fixes_applied: 0,
            duration_ms: 0,
            exit_code: 2,
        },
        diagnostics: Vec::new(),
        internal_errors: vec![JsonInternalError {
            kind: match error.kind {
                CliErrorKind::Config => "config_error".to_string(),
                CliErrorKind::Runtime | CliErrorKind::Usage => "runtime_error".to_string(),
            },
            message: error.message.clone(),
            file_path: None,
        }],
        timing: JsonTiming {
            parse_ms: 0,
            semantic_ms: 0,
            rules_ms: 0,
            fix_ms: 0,
        },
    };

    match serde_json::to_string_pretty(&payload) {
        Ok(json) => println!("{json}"),
        Err(serialize_error) => {
            eprintln!("csslint runtime_error: failed to serialize json output: {serialize_error}")
        }
    }
}

fn print_profile_report(result: &LintResult) {
    let mut files = result.file_profile.clone();
    files.sort_by(|left, right| {
        right
            .total_ms
            .total_cmp(&left.total_ms)
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut rules = result
        .rule_profile
        .iter()
        .map(|(rule_id, stats)| (rule_id.clone(), stats.clone()))
        .collect::<Vec<_>>();
    rules.sort_by(|left, right| {
        right
            .1
            .elapsed_ms
            .total_cmp(&left.1.elapsed_ms)
            .then_with(|| left.0.cmp(&right.0))
    });

    eprintln!("Profile Summary");
    eprintln!(
        "  phases: parse={}ms semantic={}ms rules={}ms fix={}ms total={}ms",
        result.timing.parse_ms,
        result.timing.semantic_ms,
        result.timing.rules_ms,
        result.timing.fix_ms,
        result.duration_ms
    );

    eprintln!("  top slow files:");
    for file in files.iter().take(5) {
        eprintln!("    - {:.2}ms  {}", file.total_ms, file.path);
    }

    eprintln!("  top expensive rules:");
    for (rule_id, stats) in rules.iter().take(5) {
        eprintln!(
            "    - {:.2}ms  {}  (diagnostics={})",
            stats.elapsed_ms, rule_id, stats.diagnostics_emitted
        );
    }
}

fn render_json(result: &LintResult) -> Result<String, serde_json::Error> {
    let payload = JsonResult {
        schema_version: 1,
        tool: "csslint",
        summary: JsonSummary {
            files_scanned: result.files_scanned,
            files_linted: result.files_linted,
            errors: result.error_count(),
            warnings: result.warning_count(),
            fixes_applied: result.fixes_applied,
            duration_ms: result.duration_ms,
            exit_code: result.exit_code(),
        },
        diagnostics: result.diagnostics.clone(),
        internal_errors: result
            .internal_errors
            .iter()
            .map(|error| JsonInternalError {
                kind: error.kind.clone(),
                message: error.message.clone(),
                file_path: error.file_path.clone(),
            })
            .collect(),
        timing: JsonTiming {
            parse_ms: result.timing.parse_ms,
            semantic_ms: result.timing.semantic_ms,
            rules_ms: result.timing.rules_ms,
            fix_ms: result.timing.fix_ms,
        },
    };

    serde_json::to_string_pretty(&payload)
}

fn render_pretty(result: &LintResult, code_frame: bool) -> String {
    render_pretty_internal(result, code_frame, false)
}

fn render_pretty_colored(result: &LintResult, code_frame: bool) -> String {
    render_pretty_internal(result, code_frame, true)
}

fn render_pretty_internal(result: &LintResult, code_frame: bool, use_color: bool) -> String {
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

        let severity = format_pretty_severity(&diagnostic.severity, use_color);
        output.push_str(&format!(
            "  {}:{}  {}  {}  {}\n",
            diagnostic.span.start_line,
            diagnostic.span.start_column,
            severity,
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

fn format_pretty_severity(severity: &str, use_color: bool) -> String {
    let label = format!("{severity:<5}");
    if !use_color {
        return label;
    }

    match severity {
        "error" => format!("\x1b[31m{label}\x1b[0m"),
        "warn" => format!("\x1b[33m{label}\x1b[0m"),
        _ => label,
    }
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

fn discover_target_files(
    target: &Path,
    explicit_ignore_path: Option<&Path>,
) -> std::io::Result<Vec<PathBuf>> {
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

    let ignore_matcher = resolve_ignore_matcher(target, explicit_ignore_path)?;

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
                if is_ignored_by_matcher(ignore_matcher.as_ref(), &path, true) {
                    continue;
                }
                pending.push(path);
                continue;
            }

            if file_type.is_file()
                && !is_ignored_by_matcher(ignore_matcher.as_ref(), &path, false)
                && is_supported_lint_file(&path)
            {
                files.push(path);
            }
        }
    }

    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    Ok(files)
}

fn resolve_ignore_matcher(
    target: &Path,
    explicit_ignore_path: Option<&Path>,
) -> std::io::Result<Option<Gitignore>> {
    let ignore_path = match explicit_ignore_path {
        Some(path) => Some(path.to_path_buf()),
        None => discover_ignore_file(target),
    };

    let Some(ignore_path) = ignore_path else {
        return Ok(None);
    };

    if !ignore_path.exists() {
        if explicit_ignore_path.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("ignore file '{}' does not exist", ignore_path.display()),
            ));
        }
        return Ok(None);
    }

    if !ignore_path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("ignore path '{}' is not a file", ignore_path.display()),
        ));
    }

    let root = if target.is_dir() {
        target
    } else {
        target.parent().unwrap_or_else(|| Path::new("."))
    };
    let mut builder = GitignoreBuilder::new(root);
    builder.add(ignore_path);

    builder
        .build()
        .map(Some)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
}

fn discover_ignore_file(target: &Path) -> Option<PathBuf> {
    let mut cursor = if target.is_dir() {
        Some(target)
    } else {
        target.parent()
    };

    while let Some(directory) = cursor {
        let candidate = directory.join(".csslintignore");
        if candidate.is_file() {
            return Some(candidate);
        }

        cursor = directory.parent();
    }

    None
}

fn is_ignored_by_matcher(matcher: Option<&Gitignore>, path: &Path, is_dir: bool) -> bool {
    matcher
        .map(|gitignore| {
            gitignore
                .matched_path_or_any_parents(path, is_dir)
                .is_ignore()
        })
        .unwrap_or(false)
}

fn is_supported_lint_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension, "css" | "vue" | "svelte"))
        .unwrap_or(false)
}

fn should_ignore_directory(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }

    matches!(name, "node_modules" | "dist" | "build")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use csslint_core::{Diagnostic, FileId, LineIndex, RuleId, Severity, Span};

    use super::{
        build_inline_suppression_plan, discover_target_files, is_diagnostic_suppressed,
        parse_cli_options, render_json, render_pretty, render_pretty_colored, CliErrorKind,
        CliOptions, LintResult, OutputFormat, PhaseTiming, RenderedDiagnostic, RenderedFix,
        RenderedSpan,
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
                ignore_path: None,
                targets_override: None,
                code_frame: false,
                profile: false,
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
    fn parse_cli_options_accepts_ignore_path_override() {
        let parsed = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--ignore-path".to_string(),
            "config/.csslintignore".to_string(),
        ])
        .expect("ignore-path override should parse");

        assert_eq!(
            parsed.ignore_path,
            Some(PathBuf::from("config/.csslintignore"))
        );
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
    fn parse_cli_options_accepts_profile_flag() {
        let parsed = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--profile".to_string(),
        ])
        .expect("profile flag should parse");

        assert!(parsed.profile);
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
    fn parse_cli_options_requires_format_value() {
        let error = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--format".to_string(),
        ])
        .expect_err("--format without value should fail");

        assert_eq!(error.kind, CliErrorKind::Usage);
        assert!(error.message.contains("missing value for --format"));
    }

    #[test]
    fn parse_cli_options_rejects_unsupported_format_value() {
        let error = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--format".to_string(),
            "yaml".to_string(),
        ])
        .expect_err("unsupported format should fail");

        assert_eq!(error.kind, CliErrorKind::Usage);
        assert!(error.message.contains("unsupported format 'yaml'"));
    }

    #[test]
    fn parse_cli_options_rejects_duplicate_config_flags() {
        let error = parse_cli_options([
            "csslint".to_string(),
            "src".to_string(),
            "--config".to_string(),
            "a.json".to_string(),
            "--config".to_string(),
            "b.json".to_string(),
        ])
        .expect_err("duplicate --config should fail");

        assert_eq!(error.kind, CliErrorKind::Usage);
        assert!(error.message.contains("--config may only be provided once"));
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
        fixture.write(".svelte-kit/generated.css", "body {}");
        fixture.write(".cache/generated.css", "body {}");

        let files = discover_target_files(fixture.path(), None).expect("discovery should succeed");
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

        let first =
            discover_target_files(fixture.path(), None).expect("first discovery should pass");
        let second =
            discover_target_files(fixture.path(), None).expect("second discovery should pass");

        assert_eq!(
            fixture.relative_paths(&first),
            fixture.relative_paths(&second)
        );
    }

    #[test]
    fn discover_target_files_honors_csslintignore_patterns() {
        let fixture = TempFixture::new("discovery-csslintignore");

        fixture.write(".csslintignore", "generated.css\nignored-dir/\n");
        fixture.write("src/kept.css", ".ok {}\n");
        fixture.write("src/generated.css", ".skip {}\n");
        fixture.write("ignored-dir/component.svelte", "<style>.skip {}</style>");

        let files =
            discover_target_files(fixture.path(), None).expect("discovery should respect ignore");
        assert_eq!(
            fixture.relative_paths(&files),
            vec!["src/kept.css".to_string()]
        );
    }

    #[test]
    fn discover_target_files_honors_explicit_ignore_path() {
        let fixture = TempFixture::new("discovery-explicit-ignore");

        fixture.write("src/keep.css", ".ok {}\n");
        fixture.write("src/skip.css", ".skip {}\n");
        fixture.write("config/custom.ignore", "src/skip.css\n");

        let ignore_path = fixture.path().join("config/custom.ignore");
        let files = discover_target_files(fixture.path(), Some(ignore_path.as_path()))
            .expect("discovery should honor explicit ignore path");

        assert_eq!(
            fixture.relative_paths(&files),
            vec!["src/keep.css".to_string()]
        );
    }

    #[test]
    fn inline_suppressions_support_block_and_line_controls() {
        let source = r#"
/* csslint-disable-next-line no_unknown_properties */
.one { colr: red; }
/* stylelint-disable-line no-unknown-properties */ .two { colr: blue; }
/* csslint-disable no_unknown_properties */
.three { colr: green; }
/* csslint-enable no_unknown_properties */
.four { colr: purple; }
"#;

        let mut diagnostics = source
            .match_indices("colr")
            .map(|(offset, _)| {
                Diagnostic::new(
                    RuleId::from("no_unknown_properties"),
                    Severity::Error,
                    "Unknown property",
                    Span::new(offset, offset + 4),
                    FileId::new(1),
                )
            })
            .collect::<Vec<_>>();

        let plan = build_inline_suppression_plan(source);
        let line_index = LineIndex::new(source);
        diagnostics.retain(|diagnostic| !is_diagnostic_suppressed(diagnostic, &line_index, &plan));

        assert_eq!(diagnostics.len(), 1);
        let (line, _) = line_index.offset_to_line_column(diagnostics[0].span.start);
        assert_eq!(line, 8);
    }

    #[test]
    fn inline_suppressions_map_stylelint_rule_ids_to_csslint_rule_ids() {
        let source = "/* stylelint-disable-next-line block-no-empty */\na {}\n";
        let offset = source.find("a {}").expect("fixture contains empty block");

        let mut diagnostics = vec![Diagnostic::new(
            RuleId::from("no_empty_rules"),
            Severity::Error,
            "Empty rule block detected",
            Span::new(offset, offset + 4),
            FileId::new(3),
        )];

        let plan = build_inline_suppression_plan(source);
        let line_index = LineIndex::new(source);
        diagnostics.retain(|diagnostic| !is_diagnostic_suppressed(diagnostic, &line_index, &plan));

        assert!(diagnostics.is_empty());
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
            internal_errors: Vec::new(),
            timing: PhaseTiming::default(),
            duration_ms: 5,
            file_profile: Vec::new(),
            rule_profile: BTreeMap::new(),
        };

        let first = render_pretty(&result, true);
        let second = render_pretty(&result, true);
        let without_frame = render_pretty(&result, false);

        assert_eq!(first, second);
        assert!(first.contains("Summary:"));
        assert!(first.contains("^"));
        assert!(!without_frame.contains("^"));
    }

    #[test]
    fn pretty_reporter_colors_warning_and_error_severities_when_enabled() {
        let result = LintResult {
            files_scanned: 1,
            files_linted: 1,
            fixes_applied: 0,
            diagnostics: vec![
                RenderedDiagnostic {
                    file_path: "src/main.css".to_string(),
                    rule_id: "no_empty_rules".to_string(),
                    severity: "warn".to_string(),
                    message: "Warning diagnostic".to_string(),
                    span: RenderedSpan {
                        start_offset: 0,
                        end_offset: 1,
                        start_line: 1,
                        start_column: 1,
                        end_line: 1,
                        end_column: 2,
                    },
                    fix: RenderedFix {
                        available: false,
                        start_offset: None,
                        end_offset: None,
                        replacement: None,
                    },
                },
                RenderedDiagnostic {
                    file_path: "src/main.css".to_string(),
                    rule_id: "no_unknown_properties".to_string(),
                    severity: "error".to_string(),
                    message: "Error diagnostic".to_string(),
                    span: RenderedSpan {
                        start_offset: 2,
                        end_offset: 3,
                        start_line: 2,
                        start_column: 1,
                        end_line: 2,
                        end_column: 2,
                    },
                    fix: RenderedFix {
                        available: false,
                        start_offset: None,
                        end_offset: None,
                        replacement: None,
                    },
                },
            ],
            file_sources: BTreeMap::new(),
            internal_errors: Vec::new(),
            timing: PhaseTiming::default(),
            duration_ms: 1,
            file_profile: Vec::new(),
            rule_profile: BTreeMap::new(),
        };

        let rendered = render_pretty_colored(&result, false);
        assert!(rendered.contains("\x1b[33mwarn "));
        assert!(rendered.contains("\x1b[31merror"));
    }

    #[test]
    fn json_reporter_emits_schema_v1_shape() {
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
                    start_offset: 0,
                    end_offset: 3,
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 4,
                },
                fix: RenderedFix {
                    available: false,
                    start_offset: None,
                    end_offset: None,
                    replacement: None,
                },
            }],
            file_sources: BTreeMap::new(),
            internal_errors: Vec::new(),
            timing: PhaseTiming {
                parse_ms: 1,
                semantic_ms: 1,
                rules_ms: 1,
                fix_ms: 0,
            },
            duration_ms: 3,
            file_profile: Vec::new(),
            rule_profile: BTreeMap::new(),
        };

        let json = render_json(&result).expect("json serialization should succeed");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("json output should be valid json");

        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["tool"], "csslint");
        assert!(value.get("summary").is_some());
        assert!(value.get("diagnostics").is_some());
        assert!(value.get("internalErrors").is_some());
        assert!(value.get("timing").is_some());
        assert!(value["diagnostics"][0].get("fix").is_some());
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
