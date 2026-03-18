#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use csslint_config::Config;
use csslint_core::{Diagnostic, FileId, TargetProfile};
use csslint_extractor::extract_styles;
use csslint_parser::{parse_style_with_options, CssParserOptions};
use csslint_rules::{run_rules_with_config_and_targets, sort_diagnostics};
use csslint_semantic::build_semantic_model;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkProtocol {
    pub warm_iterations: usize,
    pub cold_iterations: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusSummary {
    pub corpus_id: String,
    pub files: usize,
    pub total_bytes: usize,
    pub corpus_digest: String,
    pub cold_runs: Vec<IterationSummary>,
    pub warm_runs: Vec<IterationSummary>,
    pub warm_median: IterationSummary,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IterationSummary {
    pub total_ms: f64,
    pub files_per_second: f64,
    pub mb_per_second: f64,
    pub parse_ms: f64,
    pub semantic_ms: f64,
    pub rules_ms: f64,
    pub p50_file_ms: f64,
    pub p95_file_ms: f64,
    pub peak_rss_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct CorpusInput {
    pub id: String,
    pub root: PathBuf,
}

pub fn discover_corpus_inputs(root: &Path) -> Result<Vec<CorpusInput>, String> {
    if !root.exists() {
        return Err(format!("corpus root '{}' does not exist", root.display()));
    }

    let mut inputs = fs::read_dir(root)
        .map_err(|error| format!("failed to read corpus root '{}': {error}", root.display()))?
        .collect::<Result<Vec<_>, std::io::Error>>()
        .map_err(|error| format!("failed to enumerate corpora '{}': {error}", root.display()))?;

    inputs.sort_by_key(|entry| entry.file_name());

    let mut corpora = Vec::new();
    for entry in inputs {
        let path = entry.path();
        if !entry
            .file_type()
            .map_err(|error| format!("failed to inspect '{}': {error}", path.display()))?
            .is_dir()
        {
            continue;
        }

        let id = entry.file_name().to_string_lossy().to_string();
        corpora.push(CorpusInput { id, root: path });
    }

    Ok(corpora)
}

pub fn run_corpus_benchmark(
    corpus: &CorpusInput,
    protocol: BenchmarkProtocol,
) -> Result<CorpusSummary, String> {
    let files = discover_lint_files(&corpus.root)?;
    if files.is_empty() {
        return Err(format!(
            "corpus '{}' has no supported lint files (.css/.vue/.svelte)",
            corpus.id
        ));
    }

    let sources = files
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .map(|source| (path.clone(), source))
                .map_err(|error| format!("failed to read '{}': {error}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total_bytes = sources
        .iter()
        .map(|(_, source)| source.len())
        .sum::<usize>();
    let digest = digest_sources(&sources);

    let cold_runs = (0..protocol.cold_iterations)
        .map(|_| run_single_iteration(&sources))
        .collect::<Result<Vec<_>, _>>()?;

    let warm_runs = (0..protocol.warm_iterations)
        .map(|_| run_single_iteration(&sources))
        .collect::<Result<Vec<_>, _>>()?;

    let warm_median = median_iteration(&warm_runs).unwrap_or(IterationSummary {
        total_ms: 0.0,
        files_per_second: 0.0,
        mb_per_second: 0.0,
        parse_ms: 0.0,
        semantic_ms: 0.0,
        rules_ms: 0.0,
        p50_file_ms: 0.0,
        p95_file_ms: 0.0,
        peak_rss_bytes: 0,
    });

    Ok(CorpusSummary {
        corpus_id: corpus.id.clone(),
        files: files.len(),
        total_bytes,
        corpus_digest: digest,
        cold_runs,
        warm_runs,
        warm_median,
    })
}

fn run_single_iteration(sources: &[(PathBuf, String)]) -> Result<IterationSummary, String> {
    let run_started = Instant::now();
    let mut parse_ms = 0.0;
    let mut semantic_ms = 0.0;
    let mut rules_ms = 0.0;
    let mut per_file_ms = Vec::with_capacity(sources.len());
    let mut peak_rss_bytes = current_peak_rss_bytes();

    for (index, (path, source)) in sources.iter().enumerate() {
        let file_started = Instant::now();
        let file_id = FileId::new(index as u32);
        let extraction = extract_styles(file_id, path, source);
        let mut diagnostics: Vec<Diagnostic> = extraction.diagnostics;

        for style in extraction.styles {
            let parse_started = Instant::now();
            let parsed = parse_style_with_options(
                &style,
                CssParserOptions {
                    enable_recovery: false,
                    targets: TargetProfile::Defaults,
                },
            )
            .map_err(|diagnostic| {
                format!(
                    "parse failure in '{}': {}",
                    path.display(),
                    diagnostic.message
                )
            })?;
            parse_ms += parse_started.elapsed().as_secs_f64() * 1000.0;

            let semantic_started = Instant::now();
            let semantic = build_semantic_model(&parsed);
            semantic_ms += semantic_started.elapsed().as_secs_f64() * 1000.0;

            let rules_started = Instant::now();
            let rule_diagnostics = run_rules_with_config_and_targets(
                &semantic,
                &Config::default(),
                TargetProfile::Defaults,
            )
            .map_err(|config_diagnostics| {
                format!(
                    "rule config failure in '{}': {}",
                    path.display(),
                    config_diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            })?;
            rules_ms += rules_started.elapsed().as_secs_f64() * 1000.0;

            diagnostics.extend(rule_diagnostics);
        }

        sort_diagnostics(&mut diagnostics);
        per_file_ms.push(file_started.elapsed().as_secs_f64() * 1000.0);
        peak_rss_bytes = peak_rss_bytes.max(current_peak_rss_bytes());
    }

    let total_ms = run_started.elapsed().as_secs_f64() * 1000.0;
    let total_seconds = total_ms / 1000.0;
    let file_count = sources.len() as f64;
    let total_bytes = sources
        .iter()
        .map(|(_, source)| source.len())
        .sum::<usize>() as f64;

    per_file_ms.sort_by(|left, right| left.total_cmp(right));
    let p50_file_ms = percentile(&per_file_ms, 0.5);
    let p95_file_ms = percentile(&per_file_ms, 0.95);

    Ok(IterationSummary {
        total_ms,
        files_per_second: if total_seconds > 0.0 {
            file_count / total_seconds
        } else {
            0.0
        },
        mb_per_second: if total_seconds > 0.0 {
            (total_bytes / (1024.0 * 1024.0)) / total_seconds
        } else {
            0.0
        },
        parse_ms,
        semantic_ms,
        rules_ms,
        p50_file_ms,
        p95_file_ms,
        peak_rss_bytes,
    })
}

fn median_iteration(iterations: &[IterationSummary]) -> Option<IterationSummary> {
    let mut sorted = iterations.to_vec();
    sorted.sort_by(|left, right| left.total_ms.total_cmp(&right.total_ms));
    sorted.get(sorted.len() / 2).copied()
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let last_index = values.len() - 1;
    let rank = (percentile.clamp(0.0, 1.0) * last_index as f64).round() as usize;
    values[rank.min(last_index)]
}

fn discover_lint_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?
            .collect::<Result<Vec<_>, std::io::Error>>()
            .map_err(|error| format!("failed to enumerate '{}': {error}", directory.display()))?;
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| format!("failed to inspect '{}': {error}", path.display()))?;
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }

            if file_type.is_file() && is_supported_lint_file(&path) {
                files.push(path);
            }
        }
    }

    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    Ok(files)
}

fn is_supported_lint_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension, "css" | "vue" | "svelte"))
        .unwrap_or(false)
}

fn digest_sources(sources: &[(PathBuf, String)]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (path, source) in sources {
        for byte in path.to_string_lossy().as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        }
        for byte in source.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

fn current_peak_rss_bytes() -> u64 {
    let pid = std::process::id().to_string();
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", pid.as_str()])
        .output();

    let Ok(output) = output else {
        return 0;
    };

    if !output.status.success() {
        return 0;
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let Some(kilobytes) = raw.trim().parse::<u64>().ok() else {
        return 0;
    };

    kilobytes * 1024
}
