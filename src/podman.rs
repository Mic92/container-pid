use simple_error::bail;

use crate::cmd;
use crate::docker::parse_docker_output;
use crate::Container;
use crate::result::Result;

#[derive(Clone, Debug)]
pub struct Podman {}

impl Container for Podman {
    fn lookup(&self, container_id: &str) -> Result<libc::pid_t> {
        let cmd = vec![
            "podman",
            "inspect",
            "--format",
            "{{.State.Running}};{{.State.Pid}}",
            container_id,
        ];
        parse_docker_output(cmd.as_slice(), container_id)
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("podman").is_some() {
            Ok(())
        } else {
            bail!("podman not found")
        }
    }
}
