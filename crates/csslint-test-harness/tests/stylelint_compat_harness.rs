use csslint_test_harness::stylelint_compat::{run_stylelint_compat, CompatMode};

#[test]
fn compat_fast_suite_passes() {
    let summary = run_stylelint_compat(CompatMode::Fast)
        .unwrap_or_else(|error| panic!("failed to run compat-fast harness: {error}"));

    assert!(
        summary.totals.executed_cases > 0,
        "compat-fast should execute at least one case"
    );
    assert_eq!(
        summary.totals.failed,
        0,
        "compat-fast failures:\n{}",
        summary.failure_report()
    );
}

#[test]
fn compat_full_suite_passes_with_manifest_skips() {
    let summary = run_stylelint_compat(CompatMode::Full)
        .unwrap_or_else(|error| panic!("failed to run compat-full harness: {error}"));

    assert!(
        summary.totals.executed_cases > 0,
        "compat-full should execute at least one case"
    );
    assert!(
        summary.totals.skipped > 0,
        "compat-full should include explicit manifest skips"
    );
    assert_eq!(
        summary.totals.failed,
        0,
        "compat-full failures:\n{}",
        summary.failure_report()
    );
}
