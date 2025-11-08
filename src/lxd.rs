use anyhow::{bail, Context};
use libc::pid_t;
use std::process::Command;

use crate::cmd;
use crate::result::Result;
use crate::Container;

#[derive(Clone, Debug)]
pub(crate) struct Lxd {}

impl Container for Lxd {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let output = Command::new("lxc")
            .args(&["info", container_id])
            .output()
            .context("failed to execute 'lxc info'")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "lxc info command failed (exit status {}): {}",
                output.status,
                stderr.trim_end()
            );
        }

        let lines = output.stdout.split(|&c| c == b'\n');
        let mut rows = lines.map(|line| {
            let cols: Vec<&[u8]> = line.splitn(2, |&c| c == b':').collect();
            cols
        });

        if let Some(pid_row) = rows.find(|cols| cols[0] == b"Pid") {
            if pid_row.len() != 2 {
                bail!("unexpected format in 'Pid' field from lxc info");
            }
            let pid = String::from_utf8_lossy(pid_row[1]);

            pid.trim_start().parse::<pid_t>().with_context(|| {
                format!(
                    "invalid PID '{}' from lxd for container '{}'",
                    pid.trim(),
                    container_id
                )
            })
        } else {
            bail!(
                "no 'Pid' field found in lxd info output for container '{}'",
                container_id
            )
        }
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("lxc").is_some() {
            Ok(())
        } else {
            bail!("LXD runtime not found: 'lxc' command is not available")
        }
    }
}
