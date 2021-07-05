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
use crate::Container;
use simple_error::{bail, require_with, try_with};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::str::from_utf8;
use std::str::FromStr;

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
            .arg("--boot=0")
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

/// search which process has known_file open
fn find_fc_pid(vmid: &str) -> Result<libc::pid_t> {
    let known_file = PathBuf::from(format!("/tmp/log_{}_start.logs", vmid));

    // go trough all processes and seach for name=*firecracker* and open_filedescriptor=known_file
    let procs = PathBuf::from("/proc");
    for proc in try_with!(std::fs::read_dir(&procs), "cannot list {:?}", procs) {
        let proc = try_with!(proc, "cannot read entry in dir {:?}", procs);

        let pid = proc.file_name();
        let pid = pid.as_os_str().to_string_lossy();
        let pid: u64 = match u64::from_str(&pid) {
            Ok(pid) => pid,
            Err(_) => continue, // skip proc, if not a proc
        };

        // heuristic to continue early (~5%/10ms speedup)
        let cmdline = try_with!(
            fs::read(proc.path().join("cmdline")),
            "cannot read cmdline of process {}",
            pid
        );
        let cmdline = String::from_utf8_lossy(&cmdline);
        if !cmdline.contains("firecracker") {
            continue;
        }

        // search fds
        let fds = proc.path().join("fd");
        for fd in try_with!(std::fs::read_dir(&fds), "cannot list {:?}", &fds) {
            let fd = try_with!(fd, "cannot read entry in dir {:?}", &fds);
            // symlink dst = file that is open for this proc
            let open_file = try_with!(
                fs::read_link(fd.path()),
                "cannot read symlink {:?}",
                fd.path()
            );
            if open_file == known_file {
                return Ok(pid as libc::pid_t);
            }
        }
    }
    bail!("no process found for firecracker vm id {}", vmid);
}
