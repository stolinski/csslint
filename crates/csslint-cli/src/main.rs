#![forbid(unsafe_code)]

use std::path::Path;

use csslint_core::FileId;

fn main() {
    let _config = csslint_config::Config::default();
    let file_id = FileId::new(0);
    let source = ".btn {}";
    let extracted = csslint_extractor::extract_styles(file_id, Path::new("stdin.css"), source);

    let mut diagnostics = Vec::new();
    for style in &extracted {
        match csslint_parser::parse_style(style) {
            Ok(parsed) => {
                let semantic = csslint_semantic::build_semantic_model(&parsed);
                diagnostics.extend(csslint_rules::run_rules(&semantic));
            }
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    let fixes: Vec<_> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.fix.clone())
        .collect();
    let (_updated, _applied) = csslint_fix::apply_fixes(source, &fixes);

    println!("csslint CLI scaffold: {} diagnostics", diagnostics.len());
}
