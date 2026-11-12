//! `bounty-tools balance run|compare`: see `usage` below.

use std::path::Path;
use std::process::ExitCode;
use std::{env, fs};

use bounty_tools::balance::{Report, load, run};
use bounty_tools::compare::compare;

const USAGE: &str = "usage:
  bounty-tools balance run --suite <suite.json> --out <report.json> [--case <id>]... [--revision <text>]
  bounty-tools balance compare --baseline <report.json> --candidate <report.json> [--exact]

run      plays every case of the suite with every seed and writes the report. Exit 1 when the run found problems.
compare  prints what differs between two reports. Exit 1 with --exact when any battle ended differently,
         exit 2 when the reports cannot be compared.";

/// The value after `--name`, wherever it stands; every one of them for an option given more than once.
fn values(args: &[String], name: &str) -> Vec<String> {
    args.windows(2)
        .filter(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .collect()
}

fn value(args: &[String], name: &str) -> Result<String, String> {
    values(args, name)
        .pop()
        .ok_or_else(|| format!("{name} is missing\n\n{USAGE}"))
}

fn read_report(path: &str) -> Result<Report, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

fn main_inner(args: &[String]) -> Result<ExitCode, String> {
    match (
        args.first().map(String::as_str),
        args.get(1).map(String::as_str),
    ) {
        (Some("balance"), Some("run")) => {
            let loaded = load(Path::new(&value(args, "--suite")?))?;
            let revision = values(args, "--revision")
                .pop()
                .unwrap_or_else(|| "unknown".to_owned());
            let report = run(&loaded, &values(args, "--case"), &revision)?;
            let out = value(args, "--out")?;
            let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
            fs::write(&out, text + "\n").map_err(|e| format!("{out}: {e}"))?;
            println!(
                "{} of {} planned battles run, written to {out}",
                report.coverage.executed, report.coverage.planned
            );
            for case in &report.summary {
                println!(
                    "  {}: {:?}, casualties {:?}",
                    case.case_id, case.outcomes, case.casualties
                );
            }
            for problem in &report.problems {
                eprintln!("problem: {problem}");
            }
            Ok(if report.problems.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        (Some("balance"), Some("compare")) => {
            let baseline = read_report(&value(args, "--baseline")?)?;
            let candidate = read_report(&value(args, "--candidate")?)?;
            let comparison = compare(&baseline, &candidate);
            print!("{}", comparison.text());
            Ok(if !comparison.not_comparable.is_empty() {
                ExitCode::from(2)
            } else if args.iter().any(|a| a == "--exact") && !comparison.same_results() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        _ => Err(USAGE.to_owned()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    main_inner(&args).unwrap_or_else(|message| {
        eprintln!("{message}");
        ExitCode::from(2)
    })
}
