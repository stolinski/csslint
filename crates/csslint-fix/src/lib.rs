#![forbid(unsafe_code)]

use csslint_core::Fix;

pub fn apply_fixes(source: &str, fixes: &[Fix]) -> (String, usize) {
    if fixes.is_empty() {
        return (source.to_string(), 0);
    }

    let mut updated = source.to_string();
    let mut applied = 0;
    let mut ordered = fixes.to_vec();
    ordered.sort_by(|left, right| right.span.start.cmp(&left.span.start));

    for fix in ordered {
        if fix.span.start > fix.span.end || fix.span.end > updated.len() {
            continue;
        }

        updated.replace_range(fix.span.start..fix.span.end, &fix.replacement);
        applied += 1;
    }

    (updated, applied)
}
