use std::fs;
use std::path::{Path, PathBuf};

use csslint_core::{Diagnostic, FileId, Span};

#[test]
fn component_file_fixes_stay_inside_style_regions_and_are_idempotent() {
    let fixture_root = native_fixture_root().join("shared/fix");
    let case_dirs = fixture_case_dirs(&fixture_root);
    assert!(
        !case_dirs.is_empty(),
        "expected fix fixture cases in {}",
        fixture_root.display()
    );

    for (index, case_dir) in case_dirs.iter().enumerate() {
        let input_path = fixture_input_path(case_dir);
        let source = fs::read_to_string(&input_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
        let style_spans = style_spans(&input_path, &source, FileId::new(600 + index as u32));

        let first_pass = lint_source(&input_path, &source, FileId::new(600 + index as u32));
        let first_fixable = fixable_diagnostics(&first_pass);
        assert!(
            !first_fixable.is_empty(),
            "expected fixable diagnostics for {}",
            case_dir.display()
        );

        let first_fix_run =
            run_fix_engine(FileId::new(600 + index as u32), &source, &first_fixable);
        assert_eq!(
            first_fix_run.rejected,
            0,
            "expected no rejected fix proposals for {}",
            case_dir.display()
        );
        assert!(
            first_fix_run.applied > 0,
            "expected at least one applied fix for {}",
            case_dir.display()
        );
        assert_all_fixes_within_style_spans(&first_fix_run.accepted, &style_spans, case_dir);

        let original_non_style = normalize_non_style_regions(&source);
        let fixed_non_style = normalize_non_style_regions(&first_fix_run.updated);
        assert_eq!(
            original_non_style,
            fixed_non_style,
            "non-style regions changed during fix for {}",
            case_dir.display()
        );

        let second_pass = lint_source(
            &input_path,
            &first_fix_run.updated,
            FileId::new(700 + index as u32),
        );
        let second_fixable = fixable_diagnostics(&second_pass);
        assert!(
            second_fixable.is_empty(),
            "fixable diagnostics should be resolved after first pass for {}",
            case_dir.display()
        );

        let second_fix_run = run_fix_engine(
            FileId::new(700 + index as u32),
            &first_fix_run.updated,
            &second_fixable,
        );
        assert_eq!(
            second_fix_run.applied,
            0,
            "second fix pass must be a no-op for {}",
            case_dir.display()
        );
        assert_eq!(
            second_fix_run.rejected,
            0,
            "second fix pass must not reject proposals for {}",
            case_dir.display()
        );
        assert_eq!(
            second_fix_run.updated,
            first_fix_run.updated,
            "second fix pass changed output for {}",
            case_dir.display()
        );
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

fn fixable_diagnostics(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.fix.is_some())
        .cloned()
        .collect()
}

struct FixRun {
    updated: String,
    applied: usize,
    rejected: usize,
    accepted: Vec<csslint_fix::StagedFix>,
}

fn run_fix_engine(file_id: FileId, source: &str, diagnostics: &[Diagnostic]) -> FixRun {
    let collection = csslint_fix::collect_fix_proposals_for_file(file_id, source, diagnostics);
    let staged = collection
        .staged_by_file
        .get(&file_id)
        .cloned()
        .unwrap_or_default();
    let (accepted, _dropped) = csslint_fix::resolve_file_overlaps(&staged);
    let (updated, applied) = csslint_fix::apply_resolved_fixes(source, &accepted);

    FixRun {
        updated,
        applied,
        rejected: collection.rejected.len(),
        accepted,
    }
}

fn style_spans(path: &Path, source: &str, file_id: FileId) -> Vec<Span> {
    let extraction = csslint_extractor::extract_styles(file_id, path, source);
    assert!(
        extraction.diagnostics.is_empty(),
        "unexpected extraction diagnostics for {}",
        path.display()
    );

    extraction
        .styles
        .into_iter()
        .map(|style| Span::new(style.start_offset, style.end_offset))
        .collect()
}

fn assert_all_fixes_within_style_spans(
    fixes: &[csslint_fix::StagedFix],
    style_spans: &[Span],
    case_dir: &Path,
) {
    for fix in fixes {
        let inside_style = style_spans
            .iter()
            .any(|span| fix.span.start >= span.start && fix.span.end <= span.end);
        assert!(
            inside_style,
            "fix span {}..{} escapes extracted style regions for {}",
            fix.span.start,
            fix.span.end,
            case_dir.display()
        );
    }
}

fn normalize_non_style_regions(source: &str) -> String {
    const STYLE_OPEN: &str = "<style";
    const STYLE_CLOSE: &str = "</style>";
    const SENTINEL: &str = "__STYLE_CONTENT__";

    let mut normalized = String::new();
    let mut cursor = 0usize;

    while let Some(open_start_rel) = source[cursor..].find(STYLE_OPEN) {
        let open_start = cursor + open_start_rel;
        let Some(open_end_rel) = source[open_start..].find('>') else {
            normalized.push_str(&source[cursor..]);
            return normalized;
        };
        let open_end = open_start + open_end_rel;

        let style_content_start = open_end + 1;
        let Some(close_start_rel) = source[style_content_start..].find(STYLE_CLOSE) else {
            normalized.push_str(&source[cursor..]);
            return normalized;
        };
        let close_start = style_content_start + close_start_rel;

        normalized.push_str(&source[cursor..style_content_start]);
        normalized.push_str(SENTINEL);
        normalized.push_str(STYLE_CLOSE);
        cursor = close_start + STYLE_CLOSE.len();
    }

    normalized.push_str(&source[cursor..]);
    normalized
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
