mod debug_report;

use astorion::{Context, Options, parse_verbose_with};
use chrono::NaiveDateTime;
use std::io::{self, IsTerminal, Read};

const DEFAULT_REFERENCE: &str = "2013-02-12T04:30:00";

fn main() {
    let config = match parse_args() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };

    let ctx = Context { reference_time: config.reference_time };
    let opts = Options {};
    let res = parse_verbose_with(&config.input, &ctx, &opts);
    debug_report::print_run(&config.input, &res.details, config.color);
}

struct CliConfig {
    input: String,
    reference_time: NaiveDateTime,
    color: bool,
}

fn parse_args() -> Result<CliConfig, String> {
    let mut input: Option<String> = None;
    let mut reference_time = parse_reference(DEFAULT_REFERENCE)?;
    let mut color = io::stdout().is_terminal();
    let mut args = std::env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("astorion {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--color" => color = true,
            "--no-color" => color = false,
            "--reference" => {
                let value = args.next().ok_or_else(|| "error: --reference expects a value".to_string())?;
                reference_time = parse_reference(&value)?;
            }
            "--input" | "-i" => {
                let value = args.next().ok_or_else(|| "error: --input expects a value".to_string())?;
                if input.is_some() {
                    return Err("error: input provided multiple times".to_string());
                }
                input = Some(value);
            }
            "--" => {
                let rest = args.collect::<Vec<_>>().join(" ");
                if !rest.trim().is_empty() {
                    if input.is_some() {
                        return Err("error: input provided multiple times".to_string());
                    }
                    input = Some(rest);
                }
                break;
            }
            _ if arg.starts_with("--reference=") => {
                let value = arg.trim_start_matches("--reference=");
                reference_time = parse_reference(value)?;
            }
            _ if arg.starts_with("--input=") => {
                let value = arg.trim_start_matches("--input=");
                if input.is_some() {
                    return Err("error: input provided multiple times".to_string());
                }
                input = Some(value.to_string());
            }
            _ if arg.starts_with('-') => {
                return Err(format!("error: unknown option '{arg}'"));
            }
            _ => {
                let rest = std::iter::once(arg).chain(args).collect::<Vec<_>>().join(" ");
                if input.is_some() {
                    return Err("error: input provided multiple times".to_string());
                }
                input = Some(rest);
                break;
            }
        }
    }

    let input = match input {
        Some(value) => value,
        None => read_stdin_input()?,
    };

    if input.trim().is_empty() {
        return Err(format!("error: no input provided\n\n{}", help_text()));
    }

    Ok(CliConfig { input, reference_time, color })
}

fn read_stdin_input() -> Result<String, String> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).map_err(|err| format!("error: failed to read stdin: {err}"))?;
    Ok(buffer)
}

fn parse_reference(value: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .map_err(|_| format!("error: invalid --reference '{value}' (expected YYYY-MM-DDTHH:MM:SS)"))
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> String {
    format!(
        "astorion {version}

Duckling-style parsing engine CLI.

Usage:
  astorion [OPTIONS] [--] <input...>
  astorion [OPTIONS] --input <text>

Options:
  -i, --input <text>         Input text to parse. If omitted, reads remaining args
                             or stdin when no args are provided.
  --reference <timestamp>    Reference time in YYYY-MM-DDTHH:MM:SS.
                             Default: {default_reference}
  --color                    Force ANSI color output.
  --no-color                 Disable ANSI color output.
  -h, --help                 Show this help message.
  -V, --version              Print version information.

Exit codes:
  0  Success.
  1  Internal error.
  2  Invalid arguments or missing input.
",
        version = env!("CARGO_PKG_VERSION"),
        default_reference = DEFAULT_REFERENCE
    )
}
