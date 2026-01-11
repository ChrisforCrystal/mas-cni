use anyhow::Result;
use std::env;
use std::io::Read;

mod config;
mod ipam;
mod loader;
mod netlink;
mod netns;

fn main() -> Result<()> {
    // Initialize logging (output to stderr per CNI spec)
    env_logger::init();

    // 1. Read CNI command (ADD/DEL/CHECK)
    let cni_command = env::var("CNI_COMMAND").unwrap_or_default();

    // 2. Read configuration from Stdin
    let mut stdin_data = Vec::new();
    std::io::stdin().read_to_end(&mut stdin_data)?;

    // Parse config (just to validate key fields)
    let conf: config::PluginConf = serde_json::from_slice(&stdin_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse CNI config: {}", e))?;

    match cni_command.as_str() {
        "ADD" => cmd_add(&conf, &stdin_data),
        "DEL" => cmd_del(&conf, &stdin_data),
        "CHECK" => cmd_check(&conf, &stdin_data),
        "VERSION" => cmd_version(),
        _ => {
            // Usually print "about" info if no command
            println!("CNI plugin mascni v0.2.0 (Rust Edition)");
            Ok(())
        }
    }
}

fn cmd_add(conf: &config::PluginConf, raw_config: &[u8]) -> Result<()> {
    let netns_path = env::var("CNI_NETNS")?;
    let ifname = env::var("CNI_IFNAME")?;
    let container_id = env::var("CNI_CONTAINERID")?; // Used for random name gen

    // 1. Prepare Interface Names
    // Host side veth name: typically "veth" + hash
    let host_veth_name = format!("veth{}", &container_id[..5]);
    let temp_container_veth = format!("cif{}", &container_id[..5]);

    eprintln!(
        "Rust CNI: Plumbing {} <-> {} for ns {}",
        host_veth_name, ifname, netns_path
    );

    // 2. Call IPAM to get IP (Delegate)
    // 调用 IPAM 插件（如 host-local）来获取 IP 地址。
    // CNI 插件通常不自己管理 IP 池，而是“外包”给专门的 IPAM 插件。
    let ipam_result_json = ipam::exec_add(&conf.ipam.ipam_type, raw_config)?;

    // Parse IPAM result to find the IP to assign
    // 解析 IPAM 返回的 JSON 结果，提取我们需要分配给 Pod 的 IP 信息。
    let ipam_res: serde_json::Value = serde_json::from_str(&ipam_result_json)?;
    // Extract IP (e.g. "10.99.0.5/16")
    // 获取带掩码的 IP 地址字符串，用于配置网卡。
    let ip_addr_cidr = ipam_res["ips"][0]["address"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No IP address found in IPAM result"))?;
    // Extract pure IP without CIDR
    // 提取纯 IP 地址（不带掩码），用于后续的路由配置或日志打印。
    let ip_addr = ip_addr_cidr.split('/').next().unwrap_or(ip_addr_cidr);

    // 3. Create Veth Pair
    // 创建一对 Veth 虚拟网卡接口。
    // Veth 总是成对出现，像一根管子的两端：一端在宿主机 (host_veth), 一端稍后放入容器 (temp_container_veth)。
    netlink::ip_link_add_veth(&host_veth_name, &temp_container_veth)?;

    // 4. Move container-side veth to Container Netns
    // 将管子的一端（temp_container_veth）移动到 Pod 的网络命名空间中。
    // 一旦移动进去，宿主机就看不见这个接口了，它“属于”了那个容器。
    netlink::ip_link_set_ns(&temp_container_veth, &netns_path)?;

    // 5. Configure Container Interface (Inside NetNS)
    // 切换进程的视角进入 Pod 的网络命名空间，进行内部网络配置。
    netns::with_netns(&netns_path, || {
        // a. Rename temp name to CNI_IFNAME (eth0)
        // 将临时的 veth 名字重命名为 K8s 期望的标准名字（通常是 eth0）。
        let status = std::process::Command::new("ip")
            .args(&["link", "set", &temp_container_veth, "name", &ifname])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to rename interface"));
        }

        // b. Add IP
        // 给容器内的 eth0 接口绑定 IP 地址。
        netlink::ip_addr_add(&ifname, ip_addr_cidr)?;

        // c. Set UP
        // 启动容器内的 eth0 接口。
        netlink::ip_link_up(&ifname)?;

        // d. Add Default Route
        // 配置默认路由。告诉容器：“如果你要访问外网，请把包发给网关”。
        // 网关 IP 是从 IPAM 结果中获取的（例如 10.88.0.1）。
        if let Some(ip_list) = ipam_res["ips"].as_array() {
            if let Some(first_ip) = ip_list.first() {
                if let Some(gw) = first_ip.get("gateway").and_then(|v| v.as_str()) {
                    let status = std::process::Command::new("ip")
                        .args(&["route", "add", "default", "via", gw])
                        .status()?;
                    if !status.success() {
                        eprintln!("Warn: Failed to add default route");
                    }
                }
            }
        }

        Ok(())
    })?;

    // 6. Configure Host Interface (Host Side)
    // 回到宿主机命名空间，配置管子的另一端。
    // 首先启动宿主机上的 veth 接口。
    netlink::ip_link_up(&host_veth_name)?;

    // 6.1 Assign Gateway IP to Host Veth (so Pod can ping Gateway)
    // 给宿主机 Veth 接口绑定网关 IP (例如 10.88.0.1/32)。
    // 这是一个关键技巧：虽然网关 IP 逻辑上属于整个子网，但我们需要让它在宿主机的这个接口上“存在”，
    // 这样当 Pod 发起 Ping 网关的请求时，宿主机内核才会响应。
    // 使用 /32 掩码是为了避免在宿主机上产生非预期的宽范围路由。
    if let Some(ip_list) = ipam_res["ips"].as_array() {
        if let Some(first_ip) = ip_list.first() {
            if let Some(gw) = first_ip.get("gateway").and_then(|v| v.as_str()) {
                let gw_cidr = format!("{}/32", gw);
                eprintln!(
                    "Rust CNI: Adding Gateway IP {} to {}",
                    gw_cidr, host_veth_name
                );
                if let Err(e) = netlink::ip_addr_add(&host_veth_name, &gw_cidr) {
                    eprintln!("Warn: Failed to add gateway IP to host veth: {}", e);
                }
            }
        }
    }

    // 6.5 Add Route to Container IP on Host (Essential for Host <-> Pod comms)
    // ip route add <ContainerIP> dev <HostVeth>
    // 在宿主机路由表中添加一条指向 Pod IP 的路由。
    // 告诉宿主机：“如果你要发包给 10.88.0.3 (Pod)，请把包丢进 vethxxxx 这个管子”。
    // 这是 Host 到 Pod 通信的基础保障（eBPF 的保底）。
    if let Err(e) = netlink::ip_route_add(ip_addr, &host_veth_name) {
        eprintln!("Warn: Failed to add host route: {}", e);
    }

    // 7. Attach eBPF Program & specific Route

    // Only attempt BPF loading on Linux
    #[cfg(target_os = "linux")]
    {
        let bpf_obj_path = "/opt/cni/bin/tc_redirect.o";
        // 1. Attach eBPF Program
        // 将编译好的 eBPF 程序 (tc_redirect.o) 挂载到宿主机 Veth 接口的 ingress 钩子上。
        // 这意味着：所有从该 Pod 发出来的包，在进入宿主机网络栈之前，都会先经过我们的 eBPF 程序。
        if let Err(e) = loader::attach_bpf_prog(&host_veth_name, bpf_obj_path) {
            eprintln!("Warn: Failed to attach BPF: {}", e);
            // We continue for now, as eBPF object might not be present in all tests yet
        } else {
            eprintln!("Rust CNI: eBPF attached to {}", host_veth_name);

            // 2. Update BPF Map (Register Route)
            // 获取宿主机 Veth 接口的 ifindex（数字 ID，例如 15）。
            match netlink::get_ifindex(&host_veth_name) {
                Ok(idx) => {
                    // 这里的逻辑是“注册自己”：
                    // 我们要把 <PodIP> -> <HostVethIndex> 的映射关系写入全局 BPF Map。
                    //
                    // 为什么？
                    // 假设 Pod A 想发包给 Pod B (IP: 10.88.0.5)。
                    // 1. 包从 Pod A 出来，触发 Pod A 对应 Veth 上的 eBPF 程序。
                    // 2. eBPF 程序查 Map：key=10.88.0.5。
                    // 3. 如果我们在 Map 里找到了 10.88.0.5 对应的 ifindex 是 20 (即 Pod B 的 Veth)，
                    //    eBPF 就可以直接把包 redirect 到 index 20。
                    //
                    // 所以，每个 Pod 启动时，必须把“我是谁(IP)，我在哪(ifindex)”告诉 Map。
                    if let Err(e) = loader::add_route(ip_addr, idx) {
                        eprintln!("Warn: Failed to update BPF map: {}", e);
                    } else {
                        eprintln!("Rust CNI: Route added {} -> ifindex {}", ip_addr, idx);
                    }
                }
                Err(e) => eprintln!("Warn: Failed to get ifindex: {}", e),
            }
        }
    }

    // 8. Print Result
    print!("{}", ipam_result_json);
    Ok(())
}

fn cmd_del(conf: &config::PluginConf, raw_config: &[u8]) -> Result<()> {
    eprintln!("Rust CNI: DEL command");
    ipam::exec_del(&conf.ipam.ipam_type, raw_config)?;
    Ok(())
}

fn cmd_check(conf: &config::PluginConf, raw_config: &[u8]) -> Result<()> {
    eprintln!("Rust CNI: CHECK command");
    ipam::exec_check(&conf.ipam.ipam_type, raw_config)?;
    Ok(())
}

fn cmd_version() -> Result<()> {
    // Return supported versions
    let version_info = serde_json::json!({
        "cniVersion": "1.0.0",
        "supportedVersions": ["0.3.0", "0.3.1", "0.4.0", "1.0.0"]
    });
    println!("{}", version_info);
    Ok(())
}
