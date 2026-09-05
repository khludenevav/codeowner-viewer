//! Best-effort resolver that names the process listening on a given
//! TCP loopback port. Used to enrich the "address already in use" error
//! surfaced on the MCP page.
//!
//! Runs a single short-lived subprocess (`lsof` on Unix, PowerShell on
//! Windows). On failure — command missing, permission denied, no match —
//! returns `None` and the caller keeps the plain OS error.

use std::process::Command;
use std::time::Duration;

/// Cap the resolver so a hung `lsof`/PowerShell can never delay the
/// bind-failure UI response. The lookups normally complete in <50 ms.
const RESOLVER_TIMEOUT: Duration = Duration::from_secs(2);

/// Human-readable identifier of the process listening on `port` on
/// loopback, e.g. `"nginx (pid 12345)"`. Returns `None` if the OS tool
/// is missing, the port is not held, or the lookup times out.
pub fn describe_listener(port: u16) -> Option<String> {
    #[cfg(unix)]
    {
        run_lsof(port)
    }
    #[cfg(windows)]
    {
        run_powershell(port)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = port;
        None
    }
}

#[cfg(unix)]
fn run_lsof(port: u16) -> Option<String> {
    // `-nP` disables DNS/port-name resolution (fast, offline-safe).
    // `-iTCP:<port>` scopes to that TCP port; `-sTCP:LISTEN` filters to
    // the listening socket so a burst of accepted connections doesn't
    // add noise. `-F pcL` requests machine-readable output: one field
    // per line, prefixed with p=pid, c=command, L=login user.
    let output = run_with_timeout(
        Command::new("lsof").args([
            "-nP",
            "-F",
            "pcL",
            "-sTCP:LISTEN",
            &format!("-iTCP:{port}"),
        ]),
        RESOLVER_TIMEOUT,
    )?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (mut pid, mut command) = (None::<String>, None::<String>);
    for line in text.lines() {
        let mut chars = line.chars();
        match (chars.next(), chars.as_str()) {
            (Some('p'), rest) => pid = Some(rest.to_string()),
            (Some('c'), rest) => command = Some(rest.to_string()),
            _ => {}
        }
        if pid.is_some() && command.is_some() {
            break;
        }
    }
    match (command, pid) {
        (Some(c), Some(p)) => Some(format!("{c} (pid {p})")),
        (None, Some(p)) => Some(format!("pid {p}")),
        _ => None,
    }
}

#[cfg(windows)]
fn run_powershell(port: u16) -> Option<String> {
    // `Get-NetTCPConnection -LocalPort` is available on Windows 8+/
    // Server 2012+; if it's missing we return None and the caller
    // keeps the plain OS error message.
    let script = format!(
        "$c = Get-NetTCPConnection -LocalPort {port} -State Listen \
-ErrorAction SilentlyContinue | Select-Object -First 1; \
if ($c) {{ $p = Get-Process -Id $c.OwningProcess -ErrorAction SilentlyContinue; \
if ($p) {{ \"$($p.ProcessName) (pid $($p.Id))\" }} else {{ \"pid $($c.OwningProcess)\" }} }}"
    );
    let output = run_with_timeout(
        Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]),
        RESOLVER_TIMEOUT,
    )?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Spawn `cmd`, wait up to `timeout`, and return the collected output.
/// Returns `None` if the child cannot be spawned or the timeout fires
/// (the child is killed in that case so we don't leak it).
fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> Option<std::process::Output> {
    let mut child = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    // Poll `try_wait` at 20 ms granularity — good enough for a <2 s
    // ceiling while keeping the busy-wait cost negligible.
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return None,
        }
    }
    child.wait_with_output().ok()
}
