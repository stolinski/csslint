use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use csslint_config::{canonical_rule_id_order, Config};
use csslint_core::{Diagnostic, FileId, LineIndex, RuleId, Severity, TargetProfile};
use csslint_fix::apply_fixes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatMode {
    Fast,
    Full,
}

impl CompatMode {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "fast" => Some(Self::Fast),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatSource {
    pub repository: String,
    pub commit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatTotals {
    pub total_cases: usize,
    pub executed_cases: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub fixable_cases: usize,
    pub fix_passed: usize,
    pub fix_failed: usize,
    pub pass_rate: f64,
    pub fix_pass_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatRuleSummary {
    pub csslint_rule: String,
    pub stylelint_rules: Vec<String>,
    pub total_cases: usize,
    pub executed_cases: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub fixable_cases: usize,
    pub fix_passed: usize,
    pub fix_failed: usize,
    pub pass_rate: f64,
    pub fix_pass_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatReasonSummary {
    pub reason_code: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatFailure {
    pub stylelint_rule: String,
    pub csslint_rule: String,
    pub case_id: String,
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatSummary {
    pub schema_version: u8,
    pub mode: String,
    pub source: CompatSource,
    pub totals: CompatTotals,
    pub by_rule: Vec<CompatRuleSummary>,
    pub skip_reasons: Vec<CompatReasonSummary>,
    pub failures: Vec<CompatFailure>,
}

impl CompatSummary {
    pub fn failure_report(&self) -> String {
        self.failures
            .iter()
            .map(|failure| {
                format!(
                    "{}:{} ({}) -> {}",
                    failure.stylelint_rule, failure.case_id, failure.kind, failure.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone)]
pub struct RatchetResult {
    pub passed: bool,
    pub violations: Vec<String>,
}

pub fn evaluate_ratchet(current: &CompatSummary, baseline: &CompatSummary) -> RatchetResult {
    let mut violations = Vec::new();

    if current.mode != baseline.mode {
        violations.push(format!(
            "mode mismatch (current={}, baseline={})",
            current.mode, baseline.mode
        ));
    }

    maybe_record_pass_rate_drop(
        "global pass rate",
        current.totals.pass_rate,
        baseline.totals.pass_rate,
        &mut violations,
    );

    if baseline.totals.fixable_cases > 0 {
        maybe_record_pass_rate_drop(
            "global fix pass rate",
            current.totals.fix_pass_rate,
            baseline.totals.fix_pass_rate,
            &mut violations,
        );
    }

    let current_rules = current
        .by_rule
        .iter()
        .map(|rule| (rule.csslint_rule.as_str(), rule))
        .collect::<BTreeMap<_, _>>();

    for baseline_rule in &baseline.by_rule {
        let Some(current_rule) = current_rules.get(baseline_rule.csslint_rule.as_str()) else {
            violations.push(format!(
                "missing rule summary for {}",
                baseline_rule.csslint_rule
            ));
            continue;
        };

        maybe_record_pass_rate_drop(
            &format!("{} pass rate", baseline_rule.csslint_rule),
            current_rule.pass_rate,
            baseline_rule.pass_rate,
            &mut violations,
        );

        if baseline_rule.fixable_cases > 0 {
            maybe_record_pass_rate_drop(
                &format!("{} fix pass rate", baseline_rule.csslint_rule),
                current_rule.fix_pass_rate,
                baseline_rule.fix_pass_rate,
                &mut violations,
            );
        }
    }

    RatchetResult {
        passed: violations.is_empty(),
        violations,
    }
}

pub fn run_stylelint_compat(mode: CompatMode) -> Result<CompatSummary, String> {
    let fixtures = load_fixture_files()?;
    if fixtures.is_empty() {
        return Err("no imported compatibility fixtures found".to_string());
    }

    let skip_map = load_skip_manifest_map()?;
    let mut rule_stats = BTreeMap::<String, RuleAccumulator>::new();
    let mut reason_counts = BTreeMap::<String, usize>::new();
    let mut failures = Vec::<CompatFailure>::new();

    let first_stylelint = fixtures
        .first()
        .ok_or_else(|| "missing fixture source metadata".to_string())?;
    let source = CompatSource {
        repository: first_stylelint.stylelint.repository.clone(),
        commit: first_stylelint.stylelint.commit.clone(),
    };

    let mut total_cases = 0usize;
    let mut executed_cases = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut fixable_cases = 0usize;
    let mut fix_passed = 0usize;
    let mut fix_failed = 0usize;

    for fixture in fixtures {
        if fixture.stylelint.repository != source.repository
            || fixture.stylelint.commit != source.commit
        {
            return Err(format!(
                "fixture source mismatch detected for {}",
                fixture.stylelint.rule
            ));
        }

        let severity = parse_severity(&fixture.level)?;
        let rule_entry = rule_stats.entry(fixture.csslint_rule.clone()).or_default();
        rule_entry
            .stylelint_rules
            .insert(fixture.stylelint.rule.clone());

        for case in &fixture.cases {
            if mode == CompatMode::Fast && !case.fast {
                continue;
            }

            total_cases += 1;
            rule_entry.total_cases += 1;

            if let Some(skip_entry) =
                skip_map.get(&(fixture.stylelint.rule.clone(), case.id.clone()))
            {
                skipped += 1;
                rule_entry.skipped += 1;
                *reason_counts
                    .entry(skip_entry.reason_code.clone())
                    .or_insert(0) += 1;
                continue;
            }

            executed_cases += 1;
            rule_entry.executed_cases += 1;

            let expects_fix = case.expected.fixed.is_some();
            if expects_fix {
                fixable_cases += 1;
                rule_entry.fixable_cases += 1;
            }

            match execute_case(
                &fixture,
                case,
                severity,
                FileId::new(executed_cases as u32 + 6000),
            ) {
                Ok(()) => {
                    passed += 1;
                    rule_entry.passed += 1;
                    if expects_fix {
                        fix_passed += 1;
                        rule_entry.fix_passed += 1;
                    }
                }
                Err(message) => {
                    failed += 1;
                    rule_entry.failed += 1;
                    if expects_fix {
                        fix_failed += 1;
                        rule_entry.fix_failed += 1;
                    }
                    failures.push(CompatFailure {
                        stylelint_rule: fixture.stylelint.rule.clone(),
                        csslint_rule: fixture.csslint_rule.clone(),
                        case_id: case.id.clone(),
                        kind: case.kind.clone(),
                        message,
                    });
                }
            }
        }
    }

    let by_rule = rule_stats
        .into_iter()
        .map(|(csslint_rule, accumulator)| CompatRuleSummary {
            csslint_rule,
            stylelint_rules: accumulator.stylelint_rules.into_iter().collect(),
            total_cases: accumulator.total_cases,
            executed_cases: accumulator.executed_cases,
            passed: accumulator.passed,
            failed: accumulator.failed,
            skipped: accumulator.skipped,
            fixable_cases: accumulator.fixable_cases,
            fix_passed: accumulator.fix_passed,
            fix_failed: accumulator.fix_failed,
            pass_rate: ratio(accumulator.passed, accumulator.executed_cases),
            fix_pass_rate: ratio(accumulator.fix_passed, accumulator.fixable_cases),
        })
        .collect::<Vec<_>>();

    let skip_reasons = reason_counts
        .into_iter()
        .map(|(reason_code, count)| CompatReasonSummary { reason_code, count })
        .collect::<Vec<_>>();

    Ok(CompatSummary {
        schema_version: 1,
        mode: mode.as_str().to_string(),
        source,
        totals: CompatTotals {
            total_cases,
            executed_cases,
            passed,
            failed,
            skipped,
            fixable_cases,
            fix_passed,
            fix_failed,
            pass_rate: ratio(passed, executed_cases),
            fix_pass_rate: ratio(fix_passed, fixable_cases),
        },
        by_rule,
        skip_reasons,
        failures,
    })
}

#[derive(Debug, Default)]
struct RuleAccumulator {
    stylelint_rules: BTreeSet<String>,
    total_cases: usize,
    executed_cases: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    fixable_cases: usize,
    fix_passed: usize,
    fix_failed: usize,
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
    repository: String,
    commit: String,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SkipEntry {
    stylelint_rule: String,
    case_id: String,
    reason_code: String,
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
            if diagnostics
                .iter()
                .any(|diagnostic| diagnostic.fix.is_some())
            {
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

    if !extraction.diagnostics.is_empty() {
        let message = extraction
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("extraction diagnostics present: {message}"));
    }

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

    let suppression_plan = build_inline_suppression_plan(source);
    let line_index = LineIndex::new(source);
    diagnostics
        .retain(|diagnostic| !is_diagnostic_suppressed(diagnostic, &line_index, &suppression_plan));

    csslint_rules::sort_diagnostics(&mut diagnostics);
    Ok(diagnostics)
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
        _ => Err(format!("unknown fixture level '{raw}'")),
    }
}

fn load_fixture_files() -> Result<Vec<FixtureFile>, String> {
    let root = compat_root().join("imported");
    let mut fixture_paths = fs::read_dir(&root)
        .map_err(|error| format!("failed to read fixture root {}: {error}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<PathBuf>>();
    fixture_paths.sort();

    fixture_paths
        .into_iter()
        .map(|fixture_path| {
            let raw = fs::read_to_string(&fixture_path)
                .map_err(|error| format!("failed to read {}: {error}", fixture_path.display()))?;
            serde_json::from_str::<FixtureFile>(&raw)
                .map_err(|error| format!("failed to parse {}: {error}", fixture_path.display()))
        })
        .collect()
}

fn load_skip_manifest_map() -> Result<BTreeMap<(String, String), SkipEntry>, String> {
    let path = compat_root().join("skip-manifest.yaml");
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let manifest = serde_json::from_str::<SkipManifest>(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;

    Ok(manifest
        .skips
        .into_iter()
        .map(|entry| ((entry.stylelint_rule.clone(), entry.case_id.clone()), entry))
        .collect())
}

fn compat_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/compat/stylelint")
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn maybe_record_pass_rate_drop(label: &str, current: f64, baseline: f64, out: &mut Vec<String>) {
    if current + 1e-12 < baseline {
        out.push(format!(
            "{} regressed (current {:.4}, baseline {:.4})",
            label, current, baseline
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_inline_suppression_plan, canonicalize_suppression_rule_id,
        collect_suppression_directives, parse_rule_ids, parse_suppression_directive,
        SuppressionCommand,
    };

    #[test]
    fn maps_stylelint_rule_ids_to_csslint_rule_ids_for_suppressions() {
        assert_eq!(
            canonicalize_suppression_rule_id("block-no-empty"),
            "no_empty_rules"
        );
        assert_eq!(
            canonicalize_suppression_rule_id("property-no-vendor-prefix"),
            "no_legacy_vendor_prefixes"
        );
        assert_eq!(
            canonicalize_suppression_rule_id("selector-no-qualifying-type"),
            "no_overqualified_selectors"
        );
    }

    #[test]
    fn preserves_unknown_rule_ids_in_canonical_form() {
        assert_eq!(
            canonicalize_suppression_rule_id("custom-rule-name"),
            "custom_rule_name"
        );
    }

    #[test]
    fn parse_rule_ids_deduplicates_and_handles_all_keyword() {
        assert_eq!(
            parse_rule_ids("property-no-unknown, property-no-unknown block-no-empty"),
            vec![
                "no_empty_rules".to_string(),
                "no_unknown_properties".to_string()
            ]
        );
        assert!(parse_rule_ids("all").is_empty());
    }

    #[test]
    fn parse_suppression_directive_accepts_mixed_case_and_reason_tail() {
        let (command, rule_ids) = parse_suppression_directive(
            "  StYLeLiNt-DiSaBlE-NeXt-LiNe property-no-unknown, block-no-empty -- rationale",
        )
        .expect("directive should parse");

        assert_eq!(command, SuppressionCommand::DisableNextLine);
        assert_eq!(
            rule_ids,
            vec![
                "no_empty_rules".to_string(),
                "no_unknown_properties".to_string()
            ]
        );
    }

    #[test]
    fn nested_rule_disable_ranges_require_matching_enable_depth() {
        let source = "/* stylelint-disable property-no-unknown */\n.one { colr: red; }\n/* stylelint-disable property-no-unknown */\n.two { colr: red; }\n/* stylelint-enable property-no-unknown */\n.three { colr: red; }\n/* stylelint-enable property-no-unknown */\n.four { colr: red; }\n";
        let plan = build_inline_suppression_plan(source);
        let ranges = plan
            .rule_ranges
            .get("no_unknown_properties")
            .expect("rule range should be tracked");

        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start < ranges[0].end);
    }

    #[test]
    fn unterminated_block_disable_extends_to_end_of_source() {
        let source = "/* stylelint-disable property-no-unknown */\n.one { colr: red; }\n";
        let plan = build_inline_suppression_plan(source);
        let ranges = plan
            .rule_ranges
            .get("no_unknown_properties")
            .expect("unterminated disable should create an open-ended rule range");

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].end, source.len());
    }

    #[test]
    fn unterminated_comment_does_not_create_directives() {
        let source = "/* stylelint-disable property-no-unknown\n.one { colr: red; }\n";
        let directives = collect_suppression_directives(source);
        let plan = build_inline_suppression_plan(source);

        assert!(directives.is_empty());
        assert!(plan.all_ranges.is_empty());
        assert!(plan.rule_ranges.is_empty());
        assert!(plan.line_all.is_empty());
        assert!(plan.line_by_rule.is_empty());
    }
}
