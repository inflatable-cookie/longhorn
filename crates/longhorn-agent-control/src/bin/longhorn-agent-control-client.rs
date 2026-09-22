//! `longhorn-agent-control-client`: opt-in stdio carrier for the contract
//! 022 server (g02.044, feature `client`).
//!
//! A harness that cannot speak streamable HTTP spawns this binary as its
//! MCP server. The carrier discovers the one live instance, then proxies
//! newline-delimited JSON-RPC on stdio to one POST per message on the
//! instance's loopback `/mcp` endpoint with its bearer. It owns no
//! catalogue, registers nothing, and binds no listener.
//!
//! Exit status: 0 when the harness closes stdin; 2 for a startup failure
//! (usage, discovery, selection) with the reason on stderr.

use std::path::PathBuf;

use longhorn_agent_control::carrier::{resolve_carrier_dir, run_carrier, select_instance};

struct Args {
    discovery_dir: Option<PathBuf>,
    state_root: Option<PathBuf>,
    app_id: Option<String>,
}

fn usage() -> &'static str {
    "usage: longhorn-agent-control-client [--discovery-dir <dir>] [--state-root <dir>] [--app-id <id>]"
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut args = Args {
        discovery_dir: None,
        state_root: None,
        app_id: None,
    };
    let mut rest = &argv[1..];
    while let [flag, value, tail @ ..] = rest {
        match flag.as_str() {
            "--discovery-dir" => args.discovery_dir = Some(PathBuf::from(value)),
            "--state-root" => args.state_root = Some(PathBuf::from(value)),
            "--app-id" => args.app_id = Some(value.clone()),
            "--help" | "-h" => return Err("help".to_owned()),
            _ => return Err(format!("unknown flag {flag}")),
        }
        rest = tail;
    }
    if !rest.is_empty() {
        return Err(format!("unexpected argument {}", rest.join(" ")));
    }
    if args.discovery_dir.is_some() && args.state_root.is_some() {
        return Err("--discovery-dir and --state-root are mutually exclusive".to_owned());
    }
    Ok(args)
}

#[tokio::main]
async fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let args = match parse_args(&argv) {
        Ok(args) => args,
        Err(reason) if reason == "help" => {
            println!("{}", usage());
            return;
        }
        Err(reason) => {
            eprintln!("longhorn-agent-control-client: {reason}\n{}", usage());
            std::process::exit(2);
        }
    };

    if let Err(reason) = run(args).await {
        eprintln!("longhorn-agent-control-client: {reason}");
        std::process::exit(2);
    }
}

async fn run(args: Args) -> Result<(), String> {
    let dir = resolve_carrier_dir(args.discovery_dir, args.state_root)
        .map_err(|error| error.to_string())?;
    let selection =
        select_instance(&dir, args.app_id.as_deref()).map_err(|error| error.to_string())?;
    eprintln!(
        "longhorn-agent-control-client: fronting {} on {}",
        selection.file.app_id, selection.endpoint
    );
    run_carrier(selection, tokio::io::stdin(), tokio::io::stdout())
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}
