use csslint_test_harness::stylelint_compat::{run_stylelint_compat, CompatMode};

#[test]
fn compat_fast_suite_passes() {
    let summary = run_stylelint_compat(CompatMode::Fast)
        .unwrap_or_else(|error| panic!("failed to run compat-fast harness: {error}"));

    assert_eq!(
        summary.schema_version, 1,
        "compat summary schema version drifted"
    );
    assert_eq!(summary.mode, "fast", "compat-fast should report fast mode");
    assert!(
        summary.totals.executed_cases > 0,
        "compat-fast should execute at least one case"
    );
    assert_eq!(
        summary.totals.executed_cases + summary.totals.skipped,
        summary.totals.total_cases,
        "compat-fast totals should be internally consistent"
    );
    assert_eq!(
        summary.totals.passed + summary.totals.failed,
        summary.totals.executed_cases,
        "compat-fast pass/fail accounting should match executed cases"
    );
    assert_eq!(
        summary.totals.fix_passed + summary.totals.fix_failed,
        summary.totals.fixable_cases,
        "compat-fast fix accounting should match fixable cases"
    );
    assert!(
        !summary.by_rule.is_empty(),
        "compat-fast should return per-rule summaries"
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

    assert_eq!(
        summary.schema_version, 1,
        "compat summary schema version drifted"
    );
    assert_eq!(summary.mode, "full", "compat-full should report full mode");
    assert!(
        summary.totals.executed_cases > 0,
        "compat-full should execute at least one case"
    );
    assert_eq!(
        summary.totals.executed_cases + summary.totals.skipped,
        summary.totals.total_cases,
        "compat-full totals should be internally consistent"
    );
    assert_eq!(
        summary.totals.passed + summary.totals.failed,
        summary.totals.executed_cases,
        "compat-full pass/fail accounting should match executed cases"
    );
    assert_eq!(
        summary.totals.fix_passed + summary.totals.fix_failed,
        summary.totals.fixable_cases,
        "compat-full fix accounting should match fixable cases"
    );
    assert!(
        !summary.by_rule.is_empty(),
        "compat-full should return per-rule summaries"
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
