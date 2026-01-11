#[cfg(target_os = "linux")]
use anyhow::Context;
use anyhow::{anyhow, Result};
#[cfg(target_os = "linux")]
use nix::sched::{setns, CloneFlags};
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::os::unix::io::AsFd; // Import AsFd trait
use std::path::Path;

#[cfg(target_os = "linux")]
pub struct NetNS {
    file: File,
}

#[cfg(not(target_os = "linux"))]
pub struct NetNS {}

impl NetNS {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            let file = File::open(path).context("Failed to open netns path")?;
            Ok(Self { file })
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Verify path existence just for sanity, but don't open
            if !path.as_ref().exists() {
                // return Err(anyhow!("Netns path not found (mock)"));
            }
            Ok(Self {})
        }
    }

    pub fn set(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            // nix::sched::setns arg must implement AsFd.
            // std::fs::File implements AsFd.
            setns(&self.file, CloneFlags::CLONE_NEWNET).context("Failed to setns")?;
        }
        Ok(())
    }
}

pub fn get_current_netns() -> Result<NetNS> {
    #[cfg(target_os = "linux")]
    return NetNS::open("/proc/self/ns/net");

    #[cfg(not(target_os = "linux"))]
    return Ok(NetNS {});
}

// Executes a closure in the target namespace, then switches back
pub fn with_netns<F, T>(ns_path: &str, func: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    #[cfg(target_os = "linux")]
    {
        // 1. Save current netns
        // 保存当前线程的网络命名空间（即宿主机的 Netns）。
        // 这一步至关重要，因为如果我们切进容器切不回来，后续的 CNI 操作（如写结果、调 IPAM 删除）就会在这个错误的命名空间里执行，导致灾难。
        let current_ns = get_current_netns().context("Failed to get current netns")?;

        // 2. Switch to target netns
        // 打开目标命名空间的文件（通常在 /var/run/netns/ 或者是 /proc/<pid>/ns/net）。
        // 然后调用 setns() 系统调用，让当前线程“穿越”进容器的网络世界。
        let target_ns = NetNS::open(ns_path)
            .map_err(|e| anyhow!("Failed to open target netns {}: {}", ns_path, e))?;
        target_ns
            .set()
            .context("Failed to switch to target netns")?;

        // 3. Run function
        // 在容器的命名空间里执行闭包函数。
        // 这时候执行的 `ip link set eth0 name ...` 都是对容器内的网卡生效。
        let result = func();

        // 4. Switch back
        // 穿越回来。恢复现场。
        current_ns
            .set()
            .context("Failed to switch back to original netns")?;

        result
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Mock execution on non-linux (just run it)
        eprintln!(
            "Mock: Switching to netns {} (Not supported on Mac)",
            ns_path
        );
        func()
    }
}
