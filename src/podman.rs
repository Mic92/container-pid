use crate::cmd;
use crate::docker::parse_docker_output;
use crate::Container;
use crate::Error;
use crate::RawPid;

#[derive(Clone, Debug)]
pub(crate) struct Podman {}

impl Container for Podman {
    fn lookup(&self, container_id: &str) -> Result<RawPid, Error> {
        let cmd = vec![
            "podman",
            "inspect",
            "--format",
            "{{.State.Running}};{{.State.Pid}}",
            container_id,
        ];
        parse_docker_output("podman", cmd.as_slice(), container_id)
    }
    fn check_required_tools(&self) -> Result<(), Error> {
        if cmd::which("podman").is_some() {
            Ok(())
        } else {
            Err(Error::RuntimeNotFound {
                runtime: "podman",
                tool: "podman",
            })
        }
    }
}
