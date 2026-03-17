use std::fs;
use std::panic;
use std::path::{Path, PathBuf};

use csslint_core::FileId;

#[test]
fn malformed_corpus_never_panics_and_reports_controlled_diagnostics() {
    for file in malformed_fixture_files() {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));

        let run = panic::catch_unwind(|| {
            csslint_extractor::extract_styles(FileId::new(404), &file, &source)
        });
        let result = run.unwrap_or_else(|_| panic!("extractor panicked for {}", file.display()));

        assert!(
            !result.diagnostics.is_empty(),
            "expected controlled diagnostics for malformed fixture {}",
            file.display()
        );

        for diagnostic in &result.diagnostics {
            assert!(
                matches!(
                    diagnostic.rule_id.as_str(),
                    "extractor_unclosed_style_tag"
                        | "extractor_unclosed_script_tag"
                        | "unsupported_style_lang"
                        | "unsupported_external_style_src"
                ),
                "unexpected diagnostic {} in {}",
                diagnostic.rule_id.as_str(),
                file.display()
            );
        }
    }
}

fn malformed_fixture_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/native/extractor-malformed");
    let mut files = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read malformed fixture root {}: {error}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("vue") || ext.eq_ignore_ascii_case("svelte"))
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}
