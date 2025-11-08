use anyhow::{bail, Context};
use libc::pid_t;
use std::process::Command;

use crate::cmd;
use crate::result::Result;
use crate::Container;

#[derive(Clone, Debug)]
pub(crate) struct Docker {}

pub(crate) fn parse_docker_output(cmd: &[&str], container_id: &str) -> Result<pid_t> {
    let cmd_str = cmd.join(" ");
    let output = Command::new(&cmd[0])
        .args(&cmd[1..])
        .output()
        .with_context(|| format!("failed to execute command: {}", cmd_str))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "docker command failed (exit status {}): {}\nCommand: {}",
            output.status,
            stderr.trim_end(),
            cmd_str
        );
    }

    let fields: Vec<&[u8]> = output.stdout.splitn(2, |c| *c == b';').collect();
    if fields.len() != 2 {
        bail!(
            "unexpected docker output format for container '{}'",
            container_id
        );
    }

    if fields[0] != b"true" {
        bail!("container '{}' is not running", container_id);
    }

    let pid = String::from_utf8_lossy(fields[1]);

    pid.trim_end().parse::<pid_t>().with_context(|| {
        format!(
            "invalid PID '{}' from docker for container '{}'",
            pid.trim_end(),
            container_id
        )
    })
}

impl Container for Docker {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let command = if cmd::which("docker-pid").is_some() {
            vec!["docker-pid", container_id]
        } else {
            vec![
                "docker",
                "inspect",
                "--format",
                "{{.State.Running}};{{.State.Pid}}",
                container_id,
            ]
        };
        parse_docker_output(command.as_slice(), container_id)
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("docker-pid").is_some() || cmd::which("docker").is_some() {
            return Ok(());
        }

        bail!("docker runtime not found: neither 'docker' nor 'docker-pid' command is available")
    }
}
