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
    // 这里等价于在 Shell 中执行以下命令（假设 ipam_type 是 "host-local"）：
    // export CNI_COMMAND=ADD (或者 DEL/CHECK)
    // export CNI_PATH=/opt/cni/bin
    // echo '{ "cniVersion": "...", "ipam": { ... } }' | /opt/cni/bin/host-local
    //
    // 关键点：
    // 1. 通过环境变量传参 (CNI_COMMAND)。
    // 2. 通过 Stdin 管道传入 JSON 配置 (child.stdin)。
    // 3. 捕获 Stdout 作为返回值 (child.stdout)，里面包含分配到的 IP。
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
