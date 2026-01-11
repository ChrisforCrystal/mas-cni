use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub fn exec_add(ipam_type: &str, stdin_data: &[u8]) -> Result<String> {
    delegate("ADD", ipam_type, stdin_data)
}

pub fn exec_del(ipam_type: &str, stdin_data: &[u8]) -> Result<String> {
    delegate("DEL", ipam_type, stdin_data)
}

pub fn exec_check(ipam_type: &str, stdin_data: &[u8]) -> Result<String> {
    delegate("CHECK", ipam_type, stdin_data)
}

fn delegate(command: &str, ipam_type: &str, stdin_data: &[u8]) -> Result<String> {
    // 1. Find plugin path (simplified: look in /opt/cni/bin)
    let plugin_path = format!("/opt/cni/bin/{}", ipam_type);

    // 2. Prepare command
    let mut child = Command::new(&plugin_path)
        .env("CNI_COMMAND", command)
        .env("CNI_PATH", "/opt/cni/bin") // Standard path
        // Pass through other CNI env vars ideally, but for MVP minimal set
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to spawn IPAM plugin {}", plugin_path))?;

    // 3. Write config to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_data)?;
    }

    // 4. Wait for output
    let output = child.wait_with_output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("IPAM plugin failed: {:?}", output.status));
    }

    let result = String::from_utf8(output.stdout)?;
    Ok(result)
}
