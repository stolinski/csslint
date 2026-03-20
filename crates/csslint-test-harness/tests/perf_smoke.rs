use std::path::Path;
use std::time::Instant;

use csslint_core::FileId;

#[test]
fn perf_smoke_pipeline_runs() {
    let source = ".one { color: red; } .two {}";

    // Warm up parser/semantic/rule code paths before timing samples.
    let warmup_diagnostics = run_pipeline_iterations(source, perf_smoke_warmup_iterations());
    assert!(
        warmup_diagnostics > 0,
        "warmup should still execute at least one rule diagnostic"
    );

    let mut samples = Vec::with_capacity(perf_smoke_samples());
    for _ in 0..perf_smoke_samples() {
        let started = Instant::now();
        let total_diagnostics = run_pipeline_iterations(source, perf_smoke_iterations());
        let elapsed = started.elapsed().as_secs_f32();

        assert!(
            total_diagnostics > 0,
            "perf smoke sample must exercise at least one rule diagnostic"
        );
        samples.push(elapsed);
    }

    let median_seconds = median(&samples);
    assert!(
        median_seconds <= perf_threshold_seconds(),
        "median perf sample {:.3}s exceeded threshold {:.3}s (samples: {:?})",
        median_seconds,
        perf_threshold_seconds(),
        samples
    );
}

fn perf_threshold_seconds() -> f32 {
    std::env::var("CSSLINT_PERF_SMOKE_MAX_SECS")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(2.0)
}

fn perf_smoke_samples() -> usize {
    env_usize("CSSLINT_PERF_SMOKE_SAMPLES", 5)
}

fn perf_smoke_iterations() -> usize {
    env_usize("CSSLINT_PERF_SMOKE_ITERATIONS", 100)
}

fn perf_smoke_warmup_iterations() -> usize {
    env_usize("CSSLINT_PERF_SMOKE_WARMUP_ITERATIONS", 20)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn run_pipeline_iterations(source: &str, iterations: usize) -> usize {
    let mut total_diagnostics = 0usize;
    for _ in 0..iterations {
        let extraction =
            csslint_extractor::extract_styles(FileId::new(1), Path::new("perf.css"), source);
        assert!(
            extraction.diagnostics.is_empty(),
            "perf fixture should not emit extraction diagnostics"
        );
        assert_eq!(
            extraction.styles.len(),
            1,
            "perf fixture should produce one style block"
        );

        for style in &extraction.styles {
            let parsed = csslint_parser::parse_style(style).expect("parse should succeed");
            let semantic = csslint_semantic::build_semantic_model(&parsed);
            total_diagnostics += csslint_rules::run_rules(&semantic).len();
        }
    }

    total_diagnostics
}

fn median(samples: &[f32]) -> f32 {
    assert!(!samples.is_empty(), "samples must not be empty");

    let mut sorted = samples.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[middle]
    } else {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    }
}
