use std::fs;
use std::path::{Path, PathBuf};

use csslint_core::FileId;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ExpectedStyle {
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
struct ExpectedCase {
    styles: Vec<ExpectedStyle>,
    diagnostics: Vec<ExpectedDiagnostic>,
}

#[test]
fn extractor_fixture_corpus_matches_expected_output() {
    let case_dirs = fixture_case_dirs();
    assert!(
        !case_dirs.is_empty(),
        "extractor fixture corpus should not be empty"
    );

    for case_dir in case_dirs {
        let input_path = fixture_input_path(&case_dir);
        let source = fs::read_to_string(&input_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
        let expected_path = case_dir.join("expected.json");
        let expected: ExpectedCase =
            serde_json::from_str(&fs::read_to_string(&expected_path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", expected_path.display())
            }))
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", expected_path.display()));

        let extracted = csslint_extractor::extract_styles(FileId::new(1), &input_path, &source);

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
                "block ordering mismatch for {}",
                case_dir.display()
            );
            assert_eq!(
                actual.framework_string(),
                expected_style.framework.as_str(),
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
                expected_style.content.as_str(),
                "content mismatch for {} style {index}",
                case_dir.display()
            );

            let source_slice = source
                .get(actual.start_offset..actual.end_offset)
                .unwrap_or_else(|| {
                    panic!(
                        "invalid offset mapping in {} style {index}",
                        case_dir.display()
                    )
                });
            assert_eq!(
                source_slice,
                actual.content,
                "offset mapping mismatch for {} style {index}",
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
                expected_diagnostic.rule_id.as_str(),
                "diagnostic rule mismatch for {} diagnostic {index}",
                case_dir.display()
            );
            assert_eq!(
                actual.severity.as_str(),
                expected_diagnostic.severity.as_str(),
                "diagnostic severity mismatch for {} diagnostic {index}",
                case_dir.display()
            );
        }
    }
}

fn fixture_case_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/native/extractor");
    let mut entries = fs::read_dir(&root)
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

    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one input.* fixture in {}",
        case_dir.display()
    );

    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("missing input file in fixture case {}", case_dir.display()))
}

trait FrameworkString {
    fn framework_string(&self) -> &'static str;
}

impl FrameworkString for csslint_extractor::ExtractedStyle {
    fn framework_string(&self) -> &'static str {
        match self.framework {
            csslint_extractor::FrameworkKind::Css => "css",
            csslint_extractor::FrameworkKind::Vue => "vue",
            csslint_extractor::FrameworkKind::Svelte => "svelte",
        }
    }
}
