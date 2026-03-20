use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use csslint_config::Config;
use csslint_core::{Diagnostic, FileId, TargetProfile};
use csslint_fix::apply_fixes;
use csslint_parser::{parse_style_with_options, CssParserOptions};
use csslint_rules::{merge_and_sort_diagnostics, run_rules_with_config_and_targets};

#[test]
fn diagnostics_and_fixes_are_deterministic_across_thread_counts() {
    let root = fixture_root().join("perf/corpora/mixed");
    let files = collect_lint_files(&root);
    assert!(
        !files.is_empty(),
        "determinism corpus must include fixture files"
    );

    let file_count = files.len();

    let baseline = run_parallel_lint(files.clone(), 1);
    assert_eq!(
        baseline.fixed_outputs.len(),
        file_count,
        "every fixture file should produce a fixed output entry"
    );

    let threads_2 = run_parallel_lint(files.clone(), 2);
    let threads_4 = run_parallel_lint(files.clone(), 4);
    let threads_8 = run_parallel_lint(files.clone(), 8);

    assert_eq!(baseline.diagnostics, threads_2.diagnostics);
    assert_eq!(baseline.diagnostics, threads_4.diagnostics);
    assert_eq!(baseline.diagnostics, threads_8.diagnostics);
    assert_eq!(baseline.fixed_outputs, threads_2.fixed_outputs);
    assert_eq!(baseline.fixed_outputs, threads_4.fixed_outputs);
    assert_eq!(baseline.fixed_outputs, threads_8.fixed_outputs);

    for _ in 0..2 {
        let repeat_threads_4 = run_parallel_lint(files.clone(), 4);
        assert_eq!(baseline.diagnostics, repeat_threads_4.diagnostics);
        assert_eq!(baseline.fixed_outputs, repeat_threads_4.fixed_outputs);
    }

    let repeated_baseline = run_parallel_lint(files, 1);
    assert_eq!(baseline.diagnostics, repeated_baseline.diagnostics);
    assert_eq!(baseline.fixed_outputs, repeated_baseline.fixed_outputs);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeterminismResult {
    diagnostics: Vec<String>,
    fixed_outputs: BTreeMap<String, String>,
}

fn run_parallel_lint(files: Vec<PathBuf>, threads: usize) -> DeterminismResult {
    let indexed_files = Arc::new(
        files
            .into_iter()
            .enumerate()
            .collect::<Vec<(usize, PathBuf)>>(),
    );
    let cursor = Arc::new(AtomicUsize::new(0));
    let diagnostics_batches = Arc::new(Mutex::new(Vec::<Vec<Diagnostic>>::new()));
    let fixed_outputs = Arc::new(Mutex::new(BTreeMap::<String, String>::new()));

    let mut handles = Vec::new();
    for _ in 0..threads.max(1) {
        let indexed_files = Arc::clone(&indexed_files);
        let cursor = Arc::clone(&cursor);
        let diagnostics_batches = Arc::clone(&diagnostics_batches);
        let fixed_outputs = Arc::clone(&fixed_outputs);

        handles.push(std::thread::spawn(move || loop {
            let index = cursor.fetch_add(1, Ordering::SeqCst);
            let Some((file_index, path)) = indexed_files.get(index) else {
                break;
            };

            let (diagnostics, fixed) = lint_file(*file_index as u32, path);
            diagnostics_batches
                .lock()
                .expect("diagnostics mutex poisoned")
                .push(diagnostics);
            fixed_outputs
                .lock()
                .expect("fixed outputs mutex poisoned")
                .insert(path.to_string_lossy().to_string(), fixed);
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    let merged = merge_and_sort_diagnostics(
        diagnostics_batches
            .lock()
            .expect("diagnostics mutex poisoned")
            .clone(),
    );

    let rendered = merged
        .iter()
        .map(|diagnostic| {
            format!(
                "{}:{}:{}:{}:{}",
                diagnostic.file_id.get(),
                diagnostic.rule_id,
                diagnostic.severity,
                diagnostic.span.start,
                diagnostic.message
            )
        })
        .collect::<Vec<_>>();

    let fixed = fixed_outputs
        .lock()
        .expect("fixed outputs mutex poisoned")
        .clone();

    DeterminismResult {
        diagnostics: rendered,
        fixed_outputs: fixed,
    }
}

fn lint_file(file_id_raw: u32, path: &Path) -> (Vec<Diagnostic>, String) {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read '{}': {error}", path.display()));
    let file_id = FileId::new(file_id_raw + 1000);
    let extraction = csslint_extractor::extract_styles(file_id, path, &source);

    let mut diagnostics = extraction.diagnostics;
    for style in extraction.styles {
        if let Ok(parsed) = parse_style_with_options(
            &style,
            CssParserOptions {
                enable_recovery: false,
                targets: TargetProfile::Defaults,
            },
        ) {
            let semantic = csslint_semantic::build_semantic_model(&parsed);
            let mut rule_diagnostics = run_rules_with_config_and_targets(
                &semantic,
                &Config::default(),
                TargetProfile::Defaults,
            )
            .expect("default config should be valid");
            diagnostics.append(&mut rule_diagnostics);
        }
    }

    let file_fixes = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.fix.clone())
        .collect::<Vec<_>>();
    let (fixed, _applied) = apply_fixes(&source, &file_fixes);
    (diagnostics, fixed)
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(csslint_test_harness::fixture_root())
}

fn collect_lint_files(root: &Path) -> Vec<PathBuf> {
    let mut files = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read '{}': {error}", root.display()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap_or_else(|error| panic!("failed to enumerate '{}': {error}", root.display()))
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| matches!(extension, "css" | "vue" | "svelte"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    files
}
