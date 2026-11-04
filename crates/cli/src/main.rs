use std::process::ExitCode;

use clap::Parser;
use cursus_cli::{Cli, run};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    let command = cli.command.name();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if json {
                let envelope = err.envelope(command);
                match serde_json::to_string(&envelope) {
                    Ok(body) => println!("{body}"),
                    Err(_) => eprintln!("error: {err}"),
                }
            } else {
                eprintln!("error: {err}");
            }

            ExitCode::from(err.exit_code())
        }
    }
}
