use anyhow::{Result, anyhow};
use std::net::Ipv4Addr;
use std::process::Command;
use std::str::FromStr;

// Attach eBPF program to the interface using TC
pub fn attach_bpf_prog(ifname: &str, obj_path: &str) -> Result<()> {
    // 1. Add clsact qdisc (needed for ingress/egress hooks)
    // tc qdisc add dev <ifname> clsact
    let status = Command::new("tc")
        .args(&["qdisc", "add", "dev", ifname, "clsact"])
        .status(); // Ignore error if already exists?

    // 2. Attach Filter (Ingress)
    // tc filter add dev <ifname> ingress bpf da obj <obj_path> sec tc_ingress
    let status = Command::new("tc")
        .args(&[
            "filter",
            "add",
            "dev",
            ifname,
            "ingress",
            "bpf",
            "da",
            "obj",
            obj_path,
            "sec",
            "tc_ingress",
        ])
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to attach BPF program to {}", ifname));
    }

    Ok(())
}

// Update the BPF map "routes" with a new entry
pub fn add_route(dest_ip: &str, ifindex: u32) -> Result<()> {
    // Convert IP string to u32 (little endian for BPF usually, depends on arch)
    // BPF helper bpf_htons implies verifying endianness.
    // Usually map keys for IP are Network Byte Order (Big Endian).
    let ip: Ipv4Addr = Ipv4Addr::from_str(dest_ip)?;
    let ip_u32 = u32::from(ip).to_be(); // Network Byte Order

    // bpftool map update name routes key <hex> value <hex>
    // Key: 4 bytes (u32)
    // Value: 4 bytes (u32)

    let key_hex = format!(
        "{:02x} {:02x} {:02x} {:02x}",
        (ip_u32 >> 24) & 0xff,
        (ip_u32 >> 16) & 0xff,
        (ip_u32 >> 8) & 0xff,
        ip_u32 & 0xff
    );

    let val_hex = format!(
        "{:02x} {:02x} {:02x} {:02x}",
        (ifindex >> 24) & 0xff,
        (ifindex >> 16) & 0xff,
        (ifindex >> 8) & 0xff,
        ifindex & 0xff
    );

    let status = Command::new("bpftool")
        .args(&["map", "update", "name", "routes", "key", "hex"])
        .args(key_hex.split_whitespace())
        .args(&["value", "hex"])
        .args(val_hex.split_whitespace())
        .status()?;

    if !status.success() {
        // Warning: this might fail if the map is not found (e.g. first pod not loaded yet?)
        // Or if multiple pods try to update concurrently.
        return Err(anyhow!(
            "Failed to update BPF map for {} -> {}",
            dest_ip,
            ifindex
        ));
    }

    Ok(())
}
