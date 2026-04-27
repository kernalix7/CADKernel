//! Smoke tests for the `cadkernel` binary CLI.
//!
//! Covers --version, --script, --mcp, error paths, and flag conflicts.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cadkernel"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn version_flag_prints_banner_and_exits_zero() {
    let out = Command::new(bin())
        .arg("--version")
        .output()
        .expect("spawn cadkernel --version");
    assert!(out.status.success(), "exit={:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CADKernel"), "stdout = {stdout:?}");
    assert!(stdout.contains("0.1.0"), "stdout = {stdout:?}");
    assert!(stdout.contains("pre-alpha"), "stdout = {stdout:?}");
}

#[test]
fn short_version_flag_prints_banner_and_exits_zero() {
    let out = Command::new(bin())
        .arg("-V")
        .output()
        .expect("spawn cadkernel -V");
    assert!(out.status.success(), "exit={:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CADKernel"), "stdout = {stdout:?}");
}

#[test]
fn script_flag_runs_hello_cad_example() {
    let script = repo_root().join("examples/lua/hello_cad.lua");
    let out = Command::new(bin())
        .arg("--script")
        .arg(&script)
        .output()
        .expect("spawn cadkernel --script");
    assert!(
        out.status.success(),
        "exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("volume"), "stdout = {stdout:?}");
    assert!(stdout.contains("24000"), "stdout = {stdout:?}");
}

#[test]
fn script_flag_missing_file_exits_nonzero_with_message() {
    let out = Command::new(bin())
        .arg("--script")
        .arg("definitely_does_not_exist_9f8a7b.lua")
        .output()
        .expect("spawn cadkernel --script");
    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("definitely_does_not_exist_9f8a7b.lua"),
        "stderr = {stderr:?}"
    );
    assert!(
        stderr.to_lowercase().contains("error"),
        "stderr = {stderr:?}"
    );
}

#[test]
fn mcp_flag_tools_list_returns_valid_jsonrpc_response() {
    let mut child = Command::new(bin())
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cadkernel --mcp");
    {
        let stdin = child.stdin.as_mut().expect("stdin piped");
        stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}\n")
            .expect("write request");
    }
    let out = child.wait_with_output().expect("wait on cadkernel --mcp");
    assert!(
        out.status.success(),
        "exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout.lines().next().expect("at least one response line");
    let parsed: serde_json::Value =
        serde_json::from_str(line).expect("response must be valid JSON");
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 1);
    let tools = parsed
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("result.tools must be an array");
    assert!(!tools.is_empty(), "tools array should not be empty");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"create_primitive"), "names = {names:?}");
    assert!(names.contains(&"boolean_operation"), "names = {names:?}");
}

#[test]
fn mcp_and_script_flags_are_mutually_exclusive() {
    let out = Command::new(bin())
        .arg("--script")
        .arg("foo.lua")
        .arg("--mcp")
        .output()
        .expect("spawn cadkernel with conflicting flags");
    assert_eq!(
        out.status.code(),
        Some(2),
        "clap should exit 2 on conflict; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "stderr = {stderr:?}"
    );
}
