//! This module uses `kubectl` to get a containerd id, uses `journalctl` to match that id to a
//! firecracker vm id and finally searches the open file-descriptors of relevant processes for one
//! belonging to that vm id.
//!
//! The user may supply the same `container_id` as for the kubnetes module, the container always
//! defaults to `user-container` because that is where the firecracker vm resides in vhive.
//!
//! Requires:
//! - journald size longer than container lifetime
//! - vhive with "debug log containerid->vmid" patch

use crate::cmd;
use crate::kubernetes as k8s;
use crate::result::Result;
use crate::vhive_fc_vmid::find_fc_pid;
use crate::Container;
use simple_error::{bail, require_with, try_with};
use std::process::Command;
use std::str::from_utf8;

#[derive(Clone, Debug)]
pub struct Vhive {}

const DEFAULT_CONTAINER: Option<&str> = Some("user-container");

impl Container for Vhive {
    fn lookup(&self, container_id: &str) -> Result<libc::pid_t> {
        let (namespace, pod_name, _) = try_with!(
            k8s::parse_userinput(container_id),
            "cannot parse user given container_id"
        );
        let containerdid = try_with!(
            k8s::get_containerd_id(namespace, pod_name, DEFAULT_CONTAINER),
            "containerd id lookup failed"
        );
        let fcvmid = try_with!(
            get_fcvmid(&containerdid),
            "cannot get firecracker vm id for containerd://{}",
            containerdid
        );
        let pid = try_with!(
            find_fc_pid(&fcvmid),
            "cannot find pid for firecracker vmID {}",
            fcvmid
        );
        Ok(pid)
    }

    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("kubectl").is_none() {
            bail!("kubectl not found")
        }
        if cmd::which("journalctl").is_none() {
            bail!("journalctl not found")
        }
        Ok(())
    }
}

/// get firecracker vm id
fn get_fcvmid(containerd_id: &str) -> Result<String> {
    // get lines from journalctl
    let keyword = format!("user-containerID={}", containerd_id);
    let arg = format!("--grep={}", keyword);
    let result = try_with!(
        Command::new("journalctl")
            .arg("-u")
            .arg("vhive")
            .arg("--no-pager")
            .arg("--boot=0") // after a reboot all firecracker VMs of vhive stay dead
            .arg("--reverse")
            .arg("-o")
            .arg("cat")
            .arg(&arg)
            .output(),
        "cannot start journalctl"
    );
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        bail!(
            "No such container? searching journal failed (ret code {:?}): {}, {}",
            result.status.code(),
            stderr,
            stdout
        );
    }

    // parse lines
    let first = try_with!(from_utf8(&result.stdout), "journal contains non-utf8")
        .splitn(2, '\n')
        .collect::<Vec<&str>>()[0]; // first line
    let vmid = require_with!(
        first.split(' ').find_map(|kv| kv.strip_prefix("vmID=")),
        "foo"
    );
    Ok(String::from(vmid))
}
