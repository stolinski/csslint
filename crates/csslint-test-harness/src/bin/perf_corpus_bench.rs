#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::PathBuf;

use csslint_test_harness::perf::{
    discover_corpus_inputs, run_corpus_benchmark, BenchmarkProtocol, CorpusSummary,
};
use serde::Serialize;

#[derive(Debug)]
struct CliOptions {
    corpus_root: PathBuf,
    output: PathBuf,
    warm_iterations: usize,
    cold_iterations: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkEnvelope {
    schema_version: u8,
    protocol: BenchmarkProtocol,
    corpora: Vec<CorpusSummary>,
}

fn main() {
    let exit_code = match parse_cli_options(env::args().skip(1)) {
        Ok(options) => match run(options) {
            Ok(()) => 0,
            Err(message) => {
                eprintln!("{message}");
                2
            }
        },
        Err(message) => {
            eprintln!("{message}");
            2
        }
    };

    std::process::exit(exit_code);
}

fn run(options: CliOptions) -> Result<(), String> {
    let corpora = discover_corpus_inputs(&options.corpus_root)?;
    if corpora.is_empty() {
        return Err(format!(
            "no corpora found under '{}'",
            options.corpus_root.display()
        ));
    }

    let protocol = BenchmarkProtocol {
        warm_iterations: options.warm_iterations,
        cold_iterations: options.cold_iterations,
    };

    let mut summaries = Vec::new();
    for corpus in &corpora {
        summaries.push(run_corpus_benchmark(corpus, protocol.clone())?);
    }

    let envelope = BenchmarkEnvelope {
        schema_version: 1,
        protocol,
        corpora: summaries,
    };

    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create output directory '{}': {error}",
                parent.display()
            )
        })?;
    }

    let rendered = serde_json::to_string_pretty(&envelope)
        .map_err(|error| format!("failed to serialize benchmark envelope: {error}"))?;
    fs::write(&options.output, format!("{rendered}\n")).map_err(|error| {
        format!(
            "failed to write benchmark summary '{}': {error}",
            options.output.display()
        )
    })?;

    println!(
        "benchmark corpus summary written to {} ({} corpus/corpora)",
        options.output.display(),
        envelope.corpora.len()
    );
    Ok(())
}

fn parse_cli_options<I>(args: I) -> Result<CliOptions, String>
where
    I: IntoIterator<Item = String>,
{
    let mut corpus_root = PathBuf::from("tests/perf/corpora");
    let mut output: Option<PathBuf> = None;
    let mut warm_iterations = 5usize;
    let mut cold_iterations = 1usize;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--corpus-root" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --corpus-root".to_string());
                };
                corpus_root = PathBuf::from(value);
            }
            "--output" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --output".to_string());
                };
                output = Some(PathBuf::from(value));
            }
            "--warm-iterations" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --warm-iterations".to_string());
                };
                warm_iterations = value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid --warm-iterations '{value}': {error}"))?;
            }
            "--cold-iterations" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --cold-iterations".to_string());
                };
                cold_iterations = value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid --cold-iterations '{value}': {error}"))?;
            }
            "-h" | "--help" => {
                return Err("usage: cargo run -p csslint-test-harness --bin perf_corpus_bench -- --output <path> [--corpus-root tests/perf/corpora] [--warm-iterations 5] [--cold-iterations 1]".to_string());
            }
            _ => {
                return Err(format!("unknown argument: {arg}"));
            }
        }
    }

    let Some(output) = output else {
        return Err("missing required --output <path> argument".to_string());
    };

    Ok(CliOptions {
        corpus_root,
        output,
        warm_iterations,
        cold_iterations,
    })
}
