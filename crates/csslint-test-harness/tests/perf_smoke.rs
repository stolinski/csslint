use std::path::Path;
use std::time::Instant;

use csslint_core::FileId;

#[test]
fn perf_smoke_pipeline_runs() {
    let source = ".one { color: red; } .two {}";
    let started = Instant::now();

    for _ in 0..100 {
        let extraction = csslint_extractor::extract_styles(FileId::new(1), Path::new("perf.css"), source);
        for style in &extraction.styles {
            let parsed = csslint_parser::parse_style(style).expect("parse should succeed");
            let semantic = csslint_semantic::build_semantic_model(&parsed);
            let _diagnostics = csslint_rules::run_rules(&semantic);
        }
    }

    assert!(started.elapsed().as_secs_f32() < 2.0);
}
