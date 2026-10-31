//! Usage: rift-coverage <openapi.json> [--implemented FILE] [--write FILE] [--strict]
//!
//! Get the spec from https://github.com/discord/discord-api-spec (specs/openapi.json).

use rift_coverage::{analyze, parse_implemented, render_markdown, spec_operations};
use std::{env, fs, process};

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    process::exit(2)
}

fn main() {
    let mut args = env::args().skip(1);
    let mut spec_path: Option<String> = None;
    let mut implemented_path = "coverage/implemented.txt".to_owned();
    let mut write: Option<String> = None;
    let mut strict = false;

    while let Some(a) = args.next() {
        match a.as_str() {
            "--implemented" => implemented_path = args.next().unwrap_or_else(|| die("--implemented needs a path")),
            "--write" => write = Some(args.next().unwrap_or_else(|| die("--write needs a path"))),
            "--strict" => strict = true,
            "-h" | "--help" => {
                println!("usage: rift-coverage <openapi.json> [--implemented FILE] [--write FILE] [--strict]");
                return;
            }
            _ if spec_path.is_none() => spec_path = Some(a.clone()),
            _ => die(&format!("unexpected argument {a:?}")),
        }
    }
    let spec_path = spec_path.unwrap_or_else(|| die("missing <openapi.json> (see --help)"));

    let spec_text = fs::read_to_string(&spec_path).unwrap_or_else(|e| die(&format!("{spec_path}: {e}")));
    let spec: serde_json::Value =
        serde_json::from_str(&spec_text).unwrap_or_else(|e| die(&format!("{spec_path} is not valid JSON: {e}")));
    let version = spec["info"]["version"].as_str().unwrap_or("unknown").to_owned();

    let ops = spec_operations(&spec).unwrap_or_else(|e| die(&e));
    let impl_text =
        fs::read_to_string(&implemented_path).unwrap_or_else(|e| die(&format!("{implemented_path}: {e}")));
    let implemented = parse_implemented(&impl_text).unwrap_or_else(|e| die(&format!("{implemented_path}: {e}")));

    let report = analyze(&ops, &implemented);
    let md = render_markdown(&report, &version);

    match write {
        Some(path) => {
            fs::write(&path, &md).unwrap_or_else(|e| die(&format!("{path}: {e}")));
            println!(
                "{} of {} operations implemented; report written to {path}",
                report.implemented, report.total
            );
        }
        None => print!("{md}"),
    }

    if !report.unknown.is_empty() {
        eprintln!("warning: {} implemented route(s) are not in the spec:", report.unknown.len());
        for (m, p) in &report.unknown {
            eprintln!("  {m} {p}");
        }
        if strict {
            process::exit(1);
        }
    }
}
