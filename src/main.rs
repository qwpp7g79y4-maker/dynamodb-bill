use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use dynamodb_bill::{clean_with, run, Options};

// SAFETY: the only `unsafe` is one read-only `fcntl(1, F_GETFD)` in a start-up constructor; the library still forbids `unsafe`.
#[allow(unsafe_code)]
mod stdout_probe;

/// Offline estimate of AWS DynamoDB storage and capacity cost from local JSON. Reads files only, no network.
#[derive(Parser, Debug)]
#[command(
    name = "dynamodb-bill",
    version,
    about = "Offline estimate of AWS DynamoDB storage and capacity cost from local JSON. Reads files only, no network.",
    after_help = "Exit codes: 0 ok, 2 input or usage error.\nPricing: https://aws.amazon.com/dynamodb/pricing/ (geraadpleegd 2026-11-09)."
)]
struct Cli {
    /// JSON export files or a directory of *.json files.
    #[arg(required = true, value_name = "PATH")]
    inputs: Vec<PathBuf>,
    /// Price file. Default: built-in us-east-1 rates from the DynamoDB pricing page (2026-11-09).
    #[arg(long, value_name = "FILE")]
    prices: Option<PathBuf>,
    /// Group rows by this Tags/Metadata key.
    #[arg(long, value_name = "KEY")]
    group_by: Option<String>,
    /// Print JSON (`schema_version` 1) instead of the text table.
    #[arg(long)]
    json: bool,
}

fn cannot_write() -> ExitCode {
    let _ = writeln!(std::io::stderr(), "dynamodb-bill: could not write output");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    // Rust replaces a closed descriptor 1 with /dev/null before `main`; the probe
    // recorded before that whether it was closed.
    if stdout_probe::stdout_was_closed_at_start() {
        return cannot_write();
    }
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            let code = if e.use_stderr() { 2 } else { 0 };
            if e.use_stderr() {
                // clap echoes arguments; clean them like every other message.
                let text = clean_with(&e.render().to_string(), true);
                let _ = write!(std::io::stderr(), "{text}");
            } else {
                let _ = e.print();
            }
            return ExitCode::from(code);
        }
    };
    match run(&Options {
        inputs: cli.inputs,
        prices: cli.prices,
        group_by: cli.group_by,
        json: cli.json,
    }) {
        Ok(text) => {
            let mut out = stdout().lock();
            if out.write_all(text.as_bytes()).is_err() || out.flush().is_err() {
                return cannot_write();
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let _ = writeln!(std::io::stderr(), "dynamodb-bill: {err}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    #[test]
    fn cli_builds() {
        crate::Cli::command().debug_assert();
    }

    #[test]
    fn help_mentions_group_by_and_prices() {
        let mut cmd = crate::Cli::command();
        let mut buf = Vec::new();
        cmd.write_long_help(&mut buf).unwrap();
        let help = String::from_utf8(buf).unwrap();
        assert!(help.contains("--group-by"));
        assert!(help.contains("--prices"));
        assert!(help.contains("dynamodb/pricing"));
    }
}
