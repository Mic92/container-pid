use libc::pid_t;
use std::process::Command;
use simple_error::{bail, try_with};

use crate::cmd;
use crate::Container;
use crate::result::{Result};

#[derive(Clone, Debug)]
pub struct Docker {}

pub fn parse_docker_output(cmd: &[&str], container_id: &str) -> Result<pid_t> {
    let output = try_with!(
        Command::new(&cmd[0]).args(&cmd[1..]).output(),
        "Running '{}' failed",
        cmd.join(" ")
    );

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Failed to list containers. '{}' exited with {}: {}",
            cmd.join(" "),
            output.status,
            stderr.trim_end()
        );
    }

    let fields: Vec<&[u8]> = output.stdout.splitn(2, |c| *c == b';').collect();
    assert!(fields.len() == 2);

    if fields[0] != b"true" {
        bail!("container '{}' is not running", container_id);
    }

    let pid = String::from_utf8_lossy(fields[1]);

    Ok(try_with!(
        pid.trim_end().parse::<pid_t>(),
        "expected valid process id from '{}', got: {}",
        cmd.join(" "),
        pid
    ))
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

        bail!("Neither docker or docker-pid was found")
    }
}
