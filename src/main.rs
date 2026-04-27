use std::io::{BufRead, Write};
use std::process::ExitCode;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "cadkernel",
    version,
    about = "CADKernel - open-source CAD software built with Rust",
    disable_version_flag = true
)]
struct Cli {
    /// Execute a Lua script (non-GUI) and print its output to stdout.
    #[arg(long, value_name = "PATH", conflicts_with = "mcp")]
    script: Option<String>,

    /// Run the stdio MCP (JSON-RPC 2.0) server: one request per stdin line, one response per stdout line.
    #[arg(long, conflicts_with = "script")]
    mcp: bool,

    /// Print the version banner and exit.
    #[arg(short = 'V', long)]
    version: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let version = env!("CARGO_PKG_VERSION");

    if cli.version {
        println!("{}", cadkernel::version_banner(version));
        return ExitCode::SUCCESS;
    }

    if let Some(path) = cli.script.as_deref() {
        return run_script(path);
    }

    if cli.mcp {
        return run_mcp_stdio();
    }

    println!("{}", cadkernel::version_banner(version));
    cadkernel_viewer::run_gui();
    ExitCode::SUCCESS
}

fn run_script(path: &str) -> ExitCode {
    let mut engine = match cadkernel_viewer::scripting::ScriptEngine::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: failed to initialise Lua engine: {e}");
            return ExitCode::from(1);
        }
    };
    match engine.execute_file(path) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: script '{path}' failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_mcp_stdio() -> ExitCode {
    let mut server = cadkernel_io::McpServer::new();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error: failed to read stdin: {e}");
                return ExitCode::from(1);
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = match server.handle_request(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: MCP dispatch failed: {e}");
                return ExitCode::from(1);
            }
        };
        if let Err(e) = writeln!(out, "{response}") {
            eprintln!("error: failed to write response: {e}");
            return ExitCode::from(1);
        }
        if let Err(e) = out.flush() {
            eprintln!("error: failed to flush stdout: {e}");
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}
