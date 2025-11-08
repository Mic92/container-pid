use anyhow::{bail, Context};
use libc::pid_t;
use std::process::Command;

use crate::cmd;
use crate::result::Result;
use crate::Container;

#[derive(Clone, Debug)]
pub(crate) struct Lxc {}

impl Container for Lxc {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let output = Command::new("lxc-info")
            .args(&["--no-humanize", "--pid", "--name", container_id])
            .output()
            .context("failed to execute 'lxc-info'")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "lxc-info command failed (exit status {}): {}",
                output.status,
                stderr.trim_start()
            );
        }

        let pid = String::from_utf8_lossy(&output.stdout);

        pid.trim_start().parse::<pid_t>().with_context(|| {
            format!(
                "invalid PID '{}' from lxc-info for container '{}'",
                pid.trim(),
                container_id
            )
        })
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("lxc-info").is_some() {
            Ok(())
        } else {
            bail!("LXC runtime not found: 'lxc-info' command is not available")
        }
    }
}
