use libc::pid_t;
use std::process::Command;
use simple_error::{try_with, bail};

use crate::cmd;
use crate::Container;
use crate::result::{Result};

#[derive(Clone, Debug)]
pub struct Lxc {}

impl Container for Lxc {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let command = format!("lxc-info --no-humanize --pid --name {}", container_id);
        let output = try_with!(
            Command::new("lxc-info")
                .args(&["--no-humanize", "--pid", "--name", container_id])
                .output(),
            "Running '{}' failed",
            command
        );

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Failed to list containers. '{}' exited with {}: {}",
                command,
                output.status,
                stderr.trim_start()
            );
        }

        let pid = String::from_utf8_lossy(&output.stdout);

        Ok(try_with!(
            pid.trim_start().parse::<pid_t>(),
            "expected valid process id from {}, got: {}",
            command,
            pid
        ))
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("lxc-info").is_some() {
            Ok(())
        } else {
            bail!("lxc-info not found")
        }
    }
}
