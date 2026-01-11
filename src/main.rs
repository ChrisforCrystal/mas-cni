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
    let ipam_result_json = ipam::exec_add(&conf.ipam.ipam_type, raw_config)?;

    // Parse IPAM result to find the IP to assign
    let ipam_res: serde_json::Value = serde_json::from_str(&ipam_result_json)?;
    // Extract IP (e.g. "10.99.0.5/16")
    let ip_addr_cidr = ipam_res["ips"][0]["address"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No IP address found in IPAM result"))?;
    // Extract pure IP without CIDR
    let ip_addr = ip_addr_cidr.split('/').next().unwrap_or(ip_addr_cidr);

    // 3. Create Veth Pair
    netlink::ip_link_add_veth(&host_veth_name, &temp_container_veth)?;

    // 4. Move container-side veth to Container Netns
    netlink::ip_link_set_ns(&temp_container_veth, &netns_path)?;

    // 5. Configure Container Interface (Inside NetNS)
    netns::with_netns(&netns_path, || {
        // a. Rename temp name to CNI_IFNAME (eth0)
        let status = std::process::Command::new("ip")
            .args(&["link", "set", &temp_container_veth, "name", &ifname])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to rename interface"));
        }

        // b. Add IP
        netlink::ip_addr_add(&ifname, ip_addr_cidr)?;

        // c. Set UP
        netlink::ip_link_up(&ifname)?;

        // d. Add Default Route
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
    netlink::ip_link_up(&host_veth_name)?;

    // 6.1 Assign Gateway IP to Host Veth (so Pod can ping Gateway)
    // We reuse the Gateway IP from IPAM (e.g. 10.88.0.1) and add it as /32
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
    if let Err(e) = netlink::ip_route_add(ip_addr, &host_veth_name) {
        eprintln!("Warn: Failed to add host route: {}", e);
    }

    // 7. Attach eBPF Program & specific Route

    // Only attempt BPF loading on Linux
    #[cfg(target_os = "linux")]
    {
        let bpf_obj_path = "/opt/cni/bin/tc_redirect.o";
        if let Err(e) = loader::attach_bpf_prog(&host_veth_name, bpf_obj_path) {
            eprintln!("Warn: Failed to attach BPF: {}", e);
            // We continue for now, as eBPF object might not be present in all tests yet
        } else {
            eprintln!("Rust CNI: eBPF attached to {}", host_veth_name);

            // Get IfIndex used for redirect
            match netlink::get_ifindex(&host_veth_name) {
                Ok(idx) => {
                    // Add route: <ContainerIP> -> <HostVethIndex>
                    // Wait, this logic is tricky.
                    // The map "routes" is shared.
                    // The key is DestIP. The Value is Target IfIndex.
                    // Packet FROM host TO container (10.99.0.5):
                    //   Ingress hook on... wait.
                    //   Host -> Container traffic usually goes via Routing Table to `veth` interface directly.
                    //   Container -> Host/Container traffic hits Ingress hook on Host Veth.

                    // So we need to enable Container A to talk to Container B.
                    // Packet leaves Container A, hits Host Veth A.
                    // TC Redirect looks up Dest IP (Container B).
                    // Map should have: <Container B IP> -> <Host Veth B Index>.

                    // So YES, we register OURSELVES into the map so OTHERS can find us.
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
