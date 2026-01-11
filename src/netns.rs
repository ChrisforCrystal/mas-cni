#[cfg(target_os = "linux")]
use anyhow::Context;
use anyhow::{Result, anyhow};
#[cfg(target_os = "linux")]
use nix::sched::{CloneFlags, setns};
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
        let current_ns = get_current_netns().context("Failed to get current netns")?;

        // 2. Switch to target netns
        let target_ns = NetNS::open(ns_path)
            .map_err(|e| anyhow!("Failed to open target netns {}: {}", ns_path, e))?;
        target_ns
            .set()
            .context("Failed to switch to target netns")?;

        // 3. Run function
        let result = func();

        // 4. Switch back
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
