use anyhow::{bail, Context};
use libc::pid_t;
use std::process::Command;

use crate::cmd;
use crate::result::Result;
use crate::Container;

#[derive(Clone, Debug)]
pub(crate) struct Containerd {}

impl Container for Containerd {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let output = Command::new("ctr")
            .args(&["task", "list"])
            .output()
            .context("failed to execute 'ctr task list'")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "containerd task list failed (exit status {}): {}",
                output.status,
                stderr.trim_end()
            );
        }

        // $ ctr task list
        // TASK    PID      STATUS
        // v2      17515    RUNNING
        // v1      14602    RUNNING
        let mut lines = output.stdout.split(|&c| c == b'\n');
        lines.next(); // skip header
        let pid_str = lines.find_map(|line| {
            let line_str = String::from_utf8_lossy(&line);
            let cols = line_str.split_whitespace().collect::<Vec<&str>>();
            if cols.len() != 3 {
                return None;
            }

            if cols[0] == container_id {
                Some(String::from(cols[1]))
            } else {
                None
            }
        });
        match pid_str {
            Some(pid_str) => pid_str.parse::<pid_t>().with_context(|| {
                format!(
                    "invalid PID '{}' from containerd for container '{}'",
                    pid_str, container_id
                )
            }),
            None => {
                bail!("no containerd task found with id '{}'", container_id)
            }
        }
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("ctr").is_some() {
            Ok(())
        } else {
            bail!("containerd runtime not found: 'ctr' command is not available")
        }
    }
}
