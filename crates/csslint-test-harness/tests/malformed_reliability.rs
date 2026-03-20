use std::fs;
use std::path::{Path, PathBuf};

use csslint_core::FileId;
use csslint_parser::{parse_style_with_options, CssParserOptions};

#[test]
fn malformed_corpus_does_not_panic_across_pipeline() {
    let root = fixture_root().join("native/extractor-malformed");
    let files = collect_fixture_files(&root);
    assert!(!files.is_empty(), "malformed corpus should not be empty");

    let mut total_style_blocks = 0usize;
    let mut total_extraction_diagnostics = 0usize;

    for (index, file_path) in files.iter().enumerate() {
        let source = fs::read_to_string(file_path)
            .unwrap_or_else(|error| panic!("failed to read '{}': {error}", file_path.display()));
        let file_id = FileId::new(index as u32 + 800);

        let extraction = std::panic::catch_unwind(|| {
            csslint_extractor::extract_styles(file_id, file_path, &source)
        })
        .unwrap_or_else(|_| panic!("extractor panicked for '{}'", file_path.display()));

        total_extraction_diagnostics += extraction.diagnostics.len();

        for style in extraction.styles {
            total_style_blocks += 1;
            let parsed = std::panic::catch_unwind(|| {
                parse_style_with_options(
                    &style,
                    CssParserOptions {
                        enable_recovery: true,
                        targets: csslint_core::TargetProfile::Defaults,
                    },
                )
            })
            .unwrap_or_else(|_| panic!("parser panicked for '{}'", file_path.display()));

            if let Ok(parsed) = parsed {
                let semantic =
                    std::panic::catch_unwind(|| csslint_semantic::build_semantic_model(&parsed))
                        .unwrap_or_else(|_| {
                            panic!("semantic builder panicked for '{}'", file_path.display())
                        });
                let _ = std::panic::catch_unwind(|| csslint_rules::run_rules(&semantic))
                    .unwrap_or_else(|_| {
                        panic!("rule engine panicked for '{}'", file_path.display())
                    });
            }
        }
    }

    assert!(
        total_style_blocks > 0 || total_extraction_diagnostics > 0,
        "malformed corpus should exercise either style parsing or extractor diagnostics"
    );
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(csslint_test_harness::fixture_root())
}

fn collect_fixture_files(root: &Path) -> Vec<PathBuf> {
    let mut files = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read fixture root '{}': {error}", root.display()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap_or_else(|error| {
            panic!(
                "failed to enumerate fixture root '{}': {error}",
                root.display()
            )
        })
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();

    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    files
}
