use libc::pid_t;
use std::process::Command;
use simple_error::{try_with, bail};

use crate::cmd;
use crate::Container;
use crate::result::Result;

#[derive(Clone, Debug)]
pub struct Nspawn {}

impl Container for Nspawn {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let command = format!("machinectl show --property=Leader {}", container_id);
        let output = try_with!(
            Command::new("machinectl")
                .args(&["show", "--property=Leader", container_id])
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
                stderr.trim_end()
            );
        }

        let fields: Vec<&[u8]> = output.stdout.splitn(2, |c| *c == b'=').collect();
        assert!(fields.len() == 2);

        let pid = String::from_utf8_lossy(fields[1]);

        Ok(try_with!(
            pid.trim_end().parse::<pid_t>(),
            "expected valid process id from {}, got: {}",
            command,
            pid
        ))
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("machinectl").is_some() {
            Ok(())
        } else {
            bail!("machinectl not found")
        }
    }
}
