use libc::pid_t;
use std::process::Command;
use simple_error::{try_with, bail};

use crate::cmd;
use crate::Container;
use crate::result::{Result};

#[derive(Clone, Debug)]
pub struct Lxd {}

impl Container for Lxd {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let command = format!("lxc info {}", container_id);
        let output = try_with!(
            Command::new("lxc").args(&["info", container_id]).output(),
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

        let lines = output.stdout.split(|&c| c == b'\n');
        let mut rows = lines.map(|line| {
            let cols: Vec<&[u8]> = line.splitn(2, |&c| c == b':').collect();
            cols
        });

        if let Some(pid_row) = rows.find(|cols| cols[0] == b"Pid") {
            assert!(pid_row.len() == 2);
            let pid = String::from_utf8_lossy(pid_row[1]);

            Ok(try_with!(
                pid.trim_start().parse::<pid_t>(),
                "expected valid process id from {}, got: {}",
                command,
                pid
            ))
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            bail!(
                "expected to find `pid=` field in output of '{}', got: {}",
                command, stdout
            )
        }
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("lxc").is_some() {
            Ok(())
        } else {
            bail!("lxc not found")
        }
    }
}
