use libc::pid_t;
use std::process::Command;
use simple_error::{try_with, bail};

use crate::cmd;
use crate::Container;
use crate::result::{Result};

#[derive(Clone, Debug)]
pub struct Containerd {}

impl Container for Containerd {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let command = "ctr task list";
        let output = try_with!(
            Command::new("ctr").args(&["task", "list"]).output(),
            "Running '{}' failed",
            command
        );

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Failed to list containers. '{}' exited with {}: {}",
                command,
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
            Some(pid_str) => {
                let pid = try_with!(
                    pid_str.parse::<pid_t>(),
                    "read invalid pid from ctr task list: '{}'",
                    pid_str
                );
                Ok(pid)
            }
            None => {
                bail!("No container with id {} found", container_id)
            }
        }
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("ctr").is_some() {
            Ok(())
        } else {
            bail!("ctr not found")
        }
    }
}
