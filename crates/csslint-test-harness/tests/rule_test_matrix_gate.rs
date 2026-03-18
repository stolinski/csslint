use std::collections::BTreeMap;
use std::path::Path;

use csslint_core::{Diagnostic, FileId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatrixContext {
    ImportedCss,
    NativeCss,
    NativeVue,
    NativeSvelte,
}

struct MatrixCase {
    context: MatrixContext,
    path: &'static str,
    source: &'static str,
}

const IMPORTED_CSS_SOURCE: &str = "
.empty {}
article.card { colr: red; display: squish; color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; }
article.card { color: blue; }
";

const NATIVE_STYLE_SOURCE: &str = "
:global(.theme-root) { color: red; }
.empty {}
article.card { colr: red; display: squish; color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; margin-left: 1rem; clip: rect(1px,2px,3px,4px); }
article.card { color: blue; }
@viewport { width: device-width; }
";

#[test]
fn required_rule_test_matrix_cells_are_green() {
    let cases = vec![
        MatrixCase {
            context: MatrixContext::ImportedCss,
            path: "imported.css",
            source: IMPORTED_CSS_SOURCE,
        },
        MatrixCase {
            context: MatrixContext::NativeCss,
            path: "native.css",
            source: NATIVE_STYLE_SOURCE,
        },
        MatrixCase {
            context: MatrixContext::NativeVue,
            path: "Fixture.vue",
            source: "<template><article class=\"card\"></article></template>\n<style scoped>\n:global(.theme-root) { color: red; }\n.empty {}\narticle.card { colr: red; display: squish; color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; margin-left: 1rem; clip: rect(1px,2px,3px,4px); }\narticle.card { color: blue; }\n@viewport { width: device-width; }\n</style>\n",
        },
        MatrixCase {
            context: MatrixContext::NativeSvelte,
            path: "Fixture.svelte",
            source: "<script>let ready = true;</script>\n<style>\n:global(.theme-root) { color: red; }\n.empty {}\narticle.card { colr: red; display: squish; color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; margin-left: 1rem; clip: rect(1px,2px,3px,4px); }\narticle.card { color: blue; }\n@viewport { width: device-width; }\n</style>\n",
        },
    ];

    let mut diagnostics_by_context = BTreeMap::<MatrixContext, Vec<Diagnostic>>::new();
    for (index, case) in cases.iter().enumerate() {
        diagnostics_by_context.insert(
            case.context,
            lint_source(case.path, case.source, FileId::new(970 + index as u32)),
        );
    }

    let all_contexts = [
        MatrixContext::ImportedCss,
        MatrixContext::NativeCss,
        MatrixContext::NativeVue,
        MatrixContext::NativeSvelte,
    ];

    for rule_id in [
        "no_unknown_properties",
        "no_invalid_values",
        "no_duplicate_selectors",
        "no_duplicate_declarations",
        "no_empty_rules",
        "no_legacy_vendor_prefixes",
        "no_overqualified_selectors",
    ] {
        for context in all_contexts {
            assert_rule_reported(&diagnostics_by_context, context, rule_id);
        }
    }

    for context in [
        MatrixContext::NativeCss,
        MatrixContext::NativeVue,
        MatrixContext::NativeSvelte,
    ] {
        assert_rule_reported(&diagnostics_by_context, context, "no_deprecated_features");
        assert_rule_reported(
            &diagnostics_by_context,
            context,
            "prefer_logical_properties",
        );
    }

    assert_rule_not_reported(
        &diagnostics_by_context,
        MatrixContext::NativeCss,
        "no_global_leaks",
    );
    assert_rule_reported(
        &diagnostics_by_context,
        MatrixContext::NativeVue,
        "no_global_leaks",
    );
    assert_rule_reported(
        &diagnostics_by_context,
        MatrixContext::NativeSvelte,
        "no_global_leaks",
    );

    for rule_id in [
        "no_duplicate_declarations",
        "no_empty_rules",
        "no_legacy_vendor_prefixes",
    ] {
        for context in all_contexts {
            assert_rule_fix_available(&diagnostics_by_context, context, rule_id);
        }
    }

    for context in [
        MatrixContext::NativeCss,
        MatrixContext::NativeVue,
        MatrixContext::NativeSvelte,
    ] {
        assert_rule_fix_available(
            &diagnostics_by_context,
            context,
            "prefer_logical_properties",
        );
    }
}

fn assert_rule_reported(
    diagnostics_by_context: &BTreeMap<MatrixContext, Vec<Diagnostic>>,
    context: MatrixContext,
    rule_id: &str,
) {
    let diagnostics = diagnostics_by_context
        .get(&context)
        .expect("missing matrix context diagnostics");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id.as_str() == rule_id),
        "{context:?} should report {rule_id}"
    );
}

fn assert_rule_not_reported(
    diagnostics_by_context: &BTreeMap<MatrixContext, Vec<Diagnostic>>,
    context: MatrixContext,
    rule_id: &str,
) {
    let diagnostics = diagnostics_by_context
        .get(&context)
        .expect("missing matrix context diagnostics");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.rule_id.as_str() != rule_id),
        "{context:?} should not report {rule_id}"
    );
}

fn assert_rule_fix_available(
    diagnostics_by_context: &BTreeMap<MatrixContext, Vec<Diagnostic>>,
    context: MatrixContext,
    rule_id: &str,
) {
    let diagnostics = diagnostics_by_context
        .get(&context)
        .expect("missing matrix context diagnostics");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id.as_str() == rule_id && diagnostic.fix.is_some()),
        "{context:?} should include fix support for {rule_id}"
    );
}

fn lint_source(path: &str, source: &str, file_id: FileId) -> Vec<Diagnostic> {
    let extraction = csslint_extractor::extract_styles(file_id, Path::new(path), source);
    let mut diagnostics = extraction.diagnostics;

    for style in extraction.styles {
        if let Ok(parsed) = csslint_parser::parse_style(&style) {
            let semantic = csslint_semantic::build_semantic_model(&parsed);
            diagnostics.extend(csslint_rules::run_rules(&semantic));
        }
    }

    diagnostics
}
