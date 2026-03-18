#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::PathBuf;

use csslint_test_harness::stylelint_compat::{
    evaluate_ratchet, run_stylelint_compat, CompatMode, CompatSummary,
};

#[derive(Debug)]
struct CliOptions {
    mode: CompatMode,
    output: PathBuf,
    baseline: Option<PathBuf>,
    enforce_ratchet: bool,
}

fn main() {
    let exit_code = match parse_cli_options(env::args().skip(1)) {
        Ok(options) => match run(options) {
            Ok(()) => 0,
            Err(AppError::Failure(message)) => {
                eprintln!("{message}");
                1
            }
            Err(AppError::Runtime(message)) => {
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

fn run(options: CliOptions) -> Result<(), AppError> {
    let summary = run_stylelint_compat(options.mode).map_err(|error| {
        AppError::Runtime(format!("failed to run compatibility harness: {error}"))
    })?;

    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::Runtime(format!(
                "failed to create output directory '{}': {error}",
                parent.display()
            ))
        })?;
    }

    let rendered_summary = serde_json::to_string_pretty(&summary).map_err(|error| {
        AppError::Runtime(format!(
            "failed to serialize compatibility summary: {error}"
        ))
    })?;
    fs::write(&options.output, format!("{rendered_summary}\n")).map_err(|error| {
        AppError::Runtime(format!(
            "failed to write compatibility summary '{}': {error}",
            options.output.display()
        ))
    })?;

    if summary.totals.failed > 0 {
        return Err(AppError::Failure(format!(
            "compatibility failures detected ({} failed case(s)):\n{}",
            summary.totals.failed,
            summary.failure_report()
        )));
    }

    if options.enforce_ratchet {
        let baseline_path = options.baseline.as_ref().ok_or_else(|| {
            AppError::Runtime(
                "--enforce-ratchet requires --baseline <path> to be provided".to_string(),
            )
        })?;

        let baseline = load_summary(baseline_path)?;
        let ratchet = evaluate_ratchet(&summary, &baseline);
        if !ratchet.passed {
            return Err(AppError::Failure(format!(
                "compatibility ratchet check failed:\n{}",
                ratchet.violations.join("\n")
            )));
        }
    }

    println!(
        "compatibility summary written to {} (mode={}, passed={}, failed={}, skipped={})",
        options.output.display(),
        summary.mode,
        summary.totals.passed,
        summary.totals.failed,
        summary.totals.skipped
    );

    Ok(())
}

fn load_summary(path: &PathBuf) -> Result<CompatSummary, AppError> {
    let raw = fs::read_to_string(path).map_err(|error| {
        AppError::Runtime(format!(
            "failed to read baseline summary '{}': {error}",
            path.display()
        ))
    })?;

    serde_json::from_str::<CompatSummary>(&raw).map_err(|error| {
        AppError::Runtime(format!(
            "failed to parse baseline summary '{}': {error}",
            path.display()
        ))
    })
}

fn parse_cli_options<I>(args: I) -> Result<CliOptions, String>
where
    I: IntoIterator<Item = String>,
{
    let mut mode = CompatMode::Full;
    let mut output: Option<PathBuf> = None;
    let mut baseline: Option<PathBuf> = None;
    let mut enforce_ratchet = false;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mode" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --mode (expected fast or full)".to_string());
                };
                mode = CompatMode::parse(&value)
                    .ok_or_else(|| format!("invalid --mode '{value}' (expected fast or full)"))?;
            }
            "--output" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --output (expected file path)".to_string());
                };
                output = Some(PathBuf::from(value));
            }
            "--baseline" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --baseline (expected file path)".to_string());
                };
                baseline = Some(PathBuf::from(value));
            }
            "--enforce-ratchet" => {
                enforce_ratchet = true;
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run -p csslint-test-harness --bin stylelint_compat_report -- --mode <fast|full> --output <path> [--baseline <path>] [--enforce-ratchet]"
                        .to_string(),
                );
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
        mode,
        output,
        baseline,
        enforce_ratchet,
    })
}

#[derive(Debug)]
enum AppError {
    Failure(String),
    Runtime(String),
}
