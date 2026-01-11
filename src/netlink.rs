use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

pub fn ip_link_add_veth(host_name: &str, container_name: &str) -> Result<()> {
    // ip link add <host_name> type veth peer name <container_name>
    let status = Command::new("ip")
        .args(&[
            "link",
            "add",
            host_name,
            "type",
            "veth",
            "peer",
            "name",
            container_name,
        ])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to create veth pair"));
    }
    Ok(())
}

pub fn ip_link_set_ns(ifname: &str, netns_path: &str) -> Result<()> {
    // ip link set <ifname> netns <netns_path>
    let status = Command::new("ip")
        .args(&["link", "set", ifname, "netns", netns_path])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "Failed to move {} to netns {}",
            ifname,
            netns_path
        ));
    }
    Ok(())
}

pub fn ip_addr_add(ifname: &str, ip: &str) -> Result<()> {
    // ip addr add <ip> dev <ifname>
    let status = Command::new("ip")
        .args(&["addr", "add", ip, "dev", ifname])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to add ip {} to {}", ip, ifname));
    }
    Ok(())
}

pub fn ip_link_up(ifname: &str) -> Result<()> {
    let status = Command::new("ip")
        .args(&["link", "set", ifname, "up"])
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to set {} up", ifname));
    }
    Ok(())
}

pub fn ip_route_add(dst_cidr: &str, dev_name: &str) -> Result<()> {
    // ip route add <dst_cidr> dev <dev_name>
    let status = Command::new("ip")
        .args(&["route", "add", dst_cidr, "dev", dev_name])
        .status()?;

    if !status.success() {
        // e.g. "File exists" is common if IPAM delegates same IP or re-run.
        // We might want to ignore it or use 'replace'.
        // For now, return error but caller can handle.
        return Err(anyhow::anyhow!(
            "Failed to add route {} dev {}",
            dst_cidr,
            dev_name
        ));
    }
    Ok(())
}

pub fn get_ifindex(ifname: &str) -> Result<u32> {
    // Read from /sys/class/net/<ifname>/ifindex
    // Linux 内核会把每个网卡的信息暴露在 /sys/class/net/ 目录下。
    // 其中 ifindex 文件就存放了该网卡的数字 ID。
    // 这种方法比写 C 代码调用 ioctl(SIOCGIFINDEX) 要简单得多，也更 Rust 友好。
    let path = format!("/sys/class/net/{}/ifindex", ifname);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read ifindex from {}", path))?;

    // 解析文件内容，去掉末尾换行符，转成 u32。
    // 例如文件内容是 "15\n"，我们就得到数字 15。
    let index = content
        .trim()
        .parse::<u32>()
        .with_context(|| format!("Failed to parse ifindex from {}", content))?;

    Ok(index)
}
