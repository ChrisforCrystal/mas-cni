use anyhow::{anyhow, Result};
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
    // 2. Attach Filter (BPF) to Ingress
    // 等价 Shell 命令：
    // tc filter add dev <ifname> ingress bpf da obj <obj_path> sec tc_ingress
    // tc filter add dev vethxxxxx ingress bpf da obj /opt/cni/bin/tc_redirect.o sec tc_ingress

    // 命令详解：
    // - tc: Traffic Control，Linux 内核自带的流量控制工具（属于 iproute2 包）。
    // - filter add: 添加一个过滤器。
    // - dev <ifname>: 指定在哪个网卡上操作（这里是宿主机侧的 veth）。
    // - ingress: 指定处理“入站”流量（即从 Pod 发出来，进入宿主机的流量）。
    // - bpf: 指定使用 BPF 程序作为过滤器。
    // - da: Direct Action，允许 BPF 程序直接返回行为码（如 TC_ACT_SHOT, TC_ACT_REDIRECT），
    //       这是 eBPF 高性能转发的关键标志。
    // - obj <obj_path>: 指定编译好的 .o 文件路径。
    // - sec tc_ingress: 指定加载 .o 文件中名为 "tc_ingress" 的 section。
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
    // 3. 将 IP 字符串 ("10.88.0.5") 转换为 u32 整数，并调整为网络字节序 (Big Endian)。
    // 网络协议标准规定：IP 地址在包头里必须是 Big Endian 存储的。
    // eBPF 程序解析包头拿到也是 Big Endian 的数据，所以我们 Map 的 Key 也要用 Big Endian。
    let ip: Ipv4Addr = Ipv4Addr::from_str(dest_ip)?;
    let ip_u32 = u32::from(ip).to_be(); // Network Byte Order

    // bpftool map update name routes key <hex> value <hex>
    // Key: 4 bytes (u32) - 目标 IP
    // Value: 4 bytes (u32) - 目标网卡 ifindex
    //
    // 这里我们手动将 u32 转换成 4 个十六进制字节，例如：
    // IP 10.88.0.5 (Hex: 0A 58 00 05) -> "0a 58 00 05"
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

    // 调用 bpftool 更新名为 "routes" 的 Map。
    // 这里的“注册”是指：注册【我自己 (当前的 Pod)】。
    //
    // 场景：CNI 正在为 Pod A (IP=10.88.0.5) 配置网络。
    // 动作：我们将 Key="10.88.0.5", Value="Pod A 的宿主机网卡 ID" 写入全局 Map。
    //
    // 目的：是为了让【别人】能找到我。
    // 当 Pod B 想发包给 Pod A 时，Pod B 的 eBPF 程序查表：
    // "我要去 10.88.0.5，Map 里有吗？" -> "有！在 ID=20 的网卡"。
    // 于是包就被直接甩给了 Pod A。
    //
    // 【常见疑问】：为什么不填容器里面的 eth0 的 ID？
    // 答：因为 eBPF 程序跑在宿主机 (Host) 内核空间。
    // 宿主机是看不到容器内部的 eth0 的（隔着 Netns 呢）。
    // 宿主机只看得到“管子”这一头（Host Veth）。
    // 只要把包塞进 Host Veth，它自然就会顺着管子流进容器里的 eth0。
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
