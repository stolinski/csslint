use std::fs;
use std::path::{Path, PathBuf};

use csslint_core::{FileId, LineIndex};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ExpectedExtractorStyle {
    framework: String,
    scoped: bool,
    module: bool,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedDiagnostic {
    rule_id: String,
    severity: String,
}

#[derive(Debug, Deserialize)]
struct ExtractorExpectedCase {
    styles: Vec<ExpectedExtractorStyle>,
    diagnostics: Vec<ExpectedDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMappingStyle {
    block_index: u32,
    framework: String,
    start_offset: usize,
    end_offset: usize,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MappingExpectedCase {
    line_endings: String,
    styles: Vec<ExpectedMappingStyle>,
}

#[test]
fn native_fixture_layout_matches_plan() {
    let root = native_fixture_root();
    for relative in [
        "vue/extractor",
        "vue/scope",
        "vue/rules",
        "svelte/extractor",
        "svelte/scope",
        "svelte/rules",
        "shared/mapping",
        "shared/fix",
    ] {
        let required = root.join(relative);
        assert!(
            required.is_dir(),
            "missing required native fixture directory {}",
            required.display()
        );
    }
}

#[test]
fn framework_extractor_fixtures_match_expected_output() {
    for fixture_root in [
        native_fixture_root().join("vue/extractor"),
        native_fixture_root().join("svelte/extractor"),
    ] {
        let case_dirs = fixture_case_dirs(&fixture_root);
        assert!(
            !case_dirs.is_empty(),
            "expected extractor fixture cases in {}",
            fixture_root.display()
        );

        for case_dir in case_dirs {
            let input_path = fixture_input_path(&case_dir);
            let source = fs::read_to_string(&input_path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
            let expected =
                read_json_fixture::<ExtractorExpectedCase>(&case_dir.join("expected.json"));

            let extracted =
                csslint_extractor::extract_styles(FileId::new(300), &input_path, &source);
            assert_eq!(
                extracted.styles.len(),
                expected.styles.len(),
                "style count mismatch for {}",
                case_dir.display()
            );
            assert_eq!(
                extracted.diagnostics.len(),
                expected.diagnostics.len(),
                "diagnostic count mismatch for {}",
                case_dir.display()
            );

            for (index, (actual, expected_style)) in extracted
                .styles
                .iter()
                .zip(expected.styles.iter())
                .enumerate()
            {
                assert_eq!(
                    actual.block_index,
                    index as u32,
                    "block ordering mismatch for {} style {index}",
                    case_dir.display()
                );
                assert_eq!(
                    actual_framework(actual),
                    expected_style.framework,
                    "framework mismatch for {} style {index}",
                    case_dir.display()
                );
                assert_eq!(
                    actual.scoped,
                    expected_style.scoped,
                    "scoped metadata mismatch for {} style {index}",
                    case_dir.display()
                );
                assert_eq!(
                    actual.module,
                    expected_style.module,
                    "module metadata mismatch for {} style {index}",
                    case_dir.display()
                );
                assert_eq!(
                    actual.content,
                    expected_style.content,
                    "content mismatch for {} style {index}",
                    case_dir.display()
                );

                let source_slice = source
                    .get(actual.start_offset..actual.end_offset)
                    .unwrap_or_else(|| {
                        panic!(
                            "invalid style span {}..{} for {} style {index}",
                            actual.start_offset,
                            actual.end_offset,
                            case_dir.display(),
                        )
                    });
                assert_eq!(
                    source_slice,
                    actual.content,
                    "source mapping mismatch for {} style {index}",
                    case_dir.display()
                );
            }

            for (index, (actual, expected_diagnostic)) in extracted
                .diagnostics
                .iter()
                .zip(expected.diagnostics.iter())
                .enumerate()
            {
                assert_eq!(
                    actual.rule_id.as_str(),
                    expected_diagnostic.rule_id,
                    "diagnostic rule mismatch for {} diagnostic {index}",
                    case_dir.display()
                );
                assert_eq!(
                    actual.severity.as_str(),
                    expected_diagnostic.severity,
                    "diagnostic severity mismatch for {} diagnostic {index}",
                    case_dir.display()
                );
            }
        }
    }
}

#[test]
fn extractor_skips_unsupported_vue_lang_block_with_error_and_keeps_css_blocks() {
    let source = concat!(
        "<template/>\n",
        "<style lang=\"scss\">$accent: red;</style>\n",
        "<style>.ok { color: red; }</style>\n",
    );
    let path = Path::new("UnsupportedLang.vue");

    let extracted = csslint_extractor::extract_styles(FileId::new(302), path, source);
    assert_eq!(
        extracted.styles.len(),
        1,
        "unsupported style language should be skipped"
    );
    assert_eq!(
        extracted.styles[0].content, ".ok { color: red; }",
        "supported css block should still be extracted"
    );
    assert_eq!(
        extracted.diagnostics.len(),
        1,
        "unsupported style language should emit one diagnostic"
    );
    assert_eq!(
        extracted.diagnostics[0].rule_id.as_str(),
        "unsupported_style_lang",
        "unexpected diagnostic rule for unsupported style language"
    );
    assert_eq!(
        extracted.diagnostics[0].severity.as_str(),
        "error",
        "unsupported style language should be reported as error"
    );
}

#[test]
fn extractor_skips_vue_src_blocks_with_warning_and_keeps_inline_css_blocks() {
    let source = concat!(
        "<template/>\n",
        "<style src=\"./remote.css\"></style>\n",
        "<style scoped>.ok { color: red; }</style>\n",
    );
    let path = Path::new("ExternalSrc.vue");

    let extracted = csslint_extractor::extract_styles(FileId::new(303), path, source);
    assert_eq!(
        extracted.styles.len(),
        1,
        "external src block should be skipped"
    );
    assert_eq!(
        extracted.styles[0].content, ".ok { color: red; }",
        "inline css block should still be extracted"
    );
    assert_eq!(
        extracted.diagnostics.len(),
        1,
        "external src block should emit one diagnostic"
    );
    assert_eq!(
        extracted.diagnostics[0].rule_id.as_str(),
        "unsupported_external_style_src",
        "unexpected diagnostic rule for external src block"
    );
    assert_eq!(
        extracted.diagnostics[0].severity.as_str(),
        "warn",
        "external src block should be reported as warning"
    );
}

#[test]
fn shared_mapping_fixtures_validate_offsets_and_line_columns() {
    let mapping_root = native_fixture_root().join("shared/mapping");
    let case_dirs = fixture_case_dirs(&mapping_root);
    assert!(
        !case_dirs.is_empty(),
        "expected mapping fixture cases in {}",
        mapping_root.display()
    );

    for case_dir in case_dirs {
        let input_path = fixture_input_path(&case_dir);
        let source = fs::read_to_string(&input_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
        let expected = read_json_fixture::<MappingExpectedCase>(&case_dir.join("expected.json"));
        let source = apply_line_endings(&source, &expected.line_endings, &case_dir);

        let extracted = csslint_extractor::extract_styles(FileId::new(301), &input_path, &source);
        assert_eq!(
            extracted.styles.len(),
            expected.styles.len(),
            "mapping fixture style count mismatch for {}",
            case_dir.display()
        );

        let index = LineIndex::new(&source);

        for (actual, expected_style) in extracted.styles.iter().zip(expected.styles.iter()) {
            assert_eq!(
                actual.block_index,
                expected_style.block_index,
                "block index mismatch for {}",
                case_dir.display()
            );
            assert_eq!(
                actual_framework(actual),
                expected_style.framework,
                "framework mismatch for {} style block {}",
                case_dir.display(),
                expected_style.block_index
            );
            assert_eq!(
                actual.start_offset,
                expected_style.start_offset,
                "start offset mismatch for {} style block {}",
                case_dir.display(),
                expected_style.block_index
            );
            assert_eq!(
                actual.end_offset,
                expected_style.end_offset,
                "end offset mismatch for {} style block {}",
                case_dir.display(),
                expected_style.block_index
            );
            assert_eq!(
                actual.content,
                expected_style.content,
                "style content mismatch for {} style block {}",
                case_dir.display(),
                expected_style.block_index
            );

            let start_line_column = index.offset_to_line_column(actual.start_offset);
            let end_line_column = index.offset_to_line_column(actual.end_offset);
            assert_eq!(
                start_line_column,
                (expected_style.start_line, expected_style.start_column),
                "start line/column mismatch for {} style block {}",
                case_dir.display(),
                expected_style.block_index
            );
            assert_eq!(
                end_line_column,
                (expected_style.end_line, expected_style.end_column),
                "end line/column mismatch for {} style block {}",
                case_dir.display(),
                expected_style.block_index
            );

            let source_slice = source
                .get(actual.start_offset..actual.end_offset)
                .unwrap_or_else(|| {
                    panic!(
                        "invalid style span {}..{} for {} style block {}",
                        actual.start_offset,
                        actual.end_offset,
                        case_dir.display(),
                        expected_style.block_index
                    )
                });
            assert_eq!(
                source_slice,
                actual.content,
                "source mapping mismatch for {} style block {}",
                case_dir.display(),
                expected_style.block_index
            );
        }
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

fn apply_line_endings(source: &str, line_endings: &str, case_dir: &Path) -> String {
    if line_endings.eq_ignore_ascii_case("lf") {
        source.to_string()
    } else if line_endings.eq_ignore_ascii_case("crlf") {
        source.replace('\n', "\r\n")
    } else {
        panic!(
            "unsupported line_endings value '{}' in {}",
            line_endings,
            case_dir.display()
        );
    }
}

fn actual_framework(style: &csslint_extractor::ExtractedStyle) -> &'static str {
    match style.framework {
        csslint_extractor::FrameworkKind::Css => "css",
        csslint_extractor::FrameworkKind::Vue => "vue",
        csslint_extractor::FrameworkKind::Svelte => "svelte",
    }
}
