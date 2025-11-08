use anyhow::{bail, Context};
use libc::pid_t;
use std::process::Command;

use crate::cmd;
use crate::result::Result;
use crate::Container;

#[derive(Clone, Debug)]
pub(crate) struct Nspawn {}

impl Container for Nspawn {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let output = Command::new("machinectl")
            .args(&["show", "--property=Leader", container_id])
            .output()
            .context("failed to execute 'machinectl show'")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "machinectl show command failed (exit status {}): {}",
                output.status,
                stderr.trim_end()
            );
        }

        let fields: Vec<&[u8]> = output.stdout.splitn(2, |c| *c == b'=').collect();
        if fields.len() != 2 {
            bail!(
                "unexpected output format from machinectl show for container '{}'",
                container_id
            );
        }

        let pid = String::from_utf8_lossy(fields[1]);

        pid.trim_end().parse::<pid_t>().with_context(|| {
            format!(
                "invalid PID '{}' from machinectl for container '{}'",
                pid.trim(),
                container_id
            )
        })
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("machinectl").is_some() {
            Ok(())
        } else {
            bail!("systemd-nspawn runtime not found: 'machinectl' command is not available")
        }
    }
}
