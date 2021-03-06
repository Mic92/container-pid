//! This module uses `kubectl` to get a containerd id and then searches cgroups for one with that
//! id as name. It returns any pid which is a member of that group.
//!
//! Possible container_id inputs:
//!
//! - `podname` to use default namespace and first container in that pod
//! - one `/`: `namespace/podname` to override default namespace
//! - two `/`: `namespace/podname/container` to be super explicit

use crate::cmd;
use crate::result::Result;
use crate::Container;
use simple_error::{bail, require_with, try_with};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::from_utf8;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Kubernetes {}

pub const DEFAULT_NAMESPACE: &str = "default";

impl Container for Kubernetes {
    /// There is many ways to do this:
    ///  - similar to command.rs: a bit looser pattern matching on /proc/$pid/cmdline
    ///  - the following:
    fn lookup(&self, container_id: &str) -> Result<libc::pid_t> {
        let (namespace, pod_name, container_name) = try_with!(
            parse_userinput(container_id),
            "cannot parse user given container_id"
        );
        let containerdid = try_with!(
            get_containerd_id(namespace, pod_name, container_name),
            "containerd id lookup failed"
        );
        let cgroup = try_with!(find_cgroup(containerdid), "cannot find matching cgroup");
        let pid = try_with!(
            get_cgroup_pid(&cgroup),
            "cannot determine a singular pid owning the cgroup {:?}",
            cgroup
        );
        Ok(pid)
    }

    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("kubectl").is_some() {
            Ok(())
        } else {
            bail!("kubectl not found")
        }
    }
}

/// allows the user to prepend the pod name with `custom-namespace/pod-name` to override the
/// namespace (`default`). By default this will take the first container of the pod. That however
/// can be overridden by appending it like `namespace/podname/container`.
pub fn parse_userinput(container_id: &str) -> Result<(&str, &str, Option<&str>)> {
    let fields = container_id.splitn(3, '/').collect::<Vec<&str>>();
    if fields.len() == 1 {
        return Ok((DEFAULT_NAMESPACE, container_id, None));
    } else if fields.len() == 2 {
        return Ok((fields[0], fields[1], None));
    } else if fields.len() == 3 {
        return Ok((fields[0], fields[1], Some(fields[2])));
    }
    unreachable!();
}

/// find `containerd://hash` id and return hash.
/// Potentially vulnerable: passes unchecked user supplied strings to command.
pub fn get_containerd_id(
    namespace: &str,
    pod_name: &str,
    container_name: Option<&str>,
) -> Result<String> {
    let jsonpath = format!("jsonpath='{{range .items[?(@.metadata.name==\"{}\")].status.containerStatuses[*]}}{{.name}}{{\"\\t\"}}{{.containerID}}{{\"\\n\"}}{{end}}'", pod_name);
    let result = try_with!(
        Command::new("kubectl")
            .arg("get")
            .arg("pod")
            .arg("-o")
            .arg(jsonpath)
            .arg("-n")
            .arg(namespace)
            .output(),
        "kubectl command cannot be spawned"
    );
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        bail!(
            "kubectl get pod request failed (ret code {:?}): {}",
            result.status.code(),
            stderr
        );
    }

    let containers = try_with!(from_utf8(&result.stdout), "response contains non-utf8");
    let containerid = containers.split('\n').find_map(|line| {
        // line = "containername\tcontainerdid"
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() != 2 {
            return None;
        }
        if let Some(name) = container_name {
            // return name-matching containerid
            if cols[0] == name {
                return Some(cols[1]);
            }
        } else {
            // return any containerid
            return Some(cols[1]);
        }
        None
    });
    let containerid = require_with!(
        containerid,
        "no container found matching {:?}",
        container_name
    );

    let containerid = require_with!(
        containerid.strip_prefix("containerd://"),
        "unexpected/unparsable containerd id"
    );
    Ok(String::from(containerid))
}

pub fn find_cgroup(containerdid: String) -> Result<PathBuf> {
    let path = visit_dirs(
        &PathBuf::from("/sys/fs/cgroup"),
        &OsString::from(containerdid),
    )?;
    Ok(path)
}

// one possible implementation of walking a directory from
// https://doc.rust-lang.org/std/fs/fn.read_dir.html
fn visit_dirs(dir: &Path, containerdid: &OsString) -> Result<PathBuf> {
    for entry in try_with!(std::fs::read_dir(dir), "cannot list {:?}", dir) {
        let entry = try_with!(entry, "cannot read entry in dir {:?}", dir);
        if &entry.file_name() == containerdid {
            return Ok(entry.path());
        }
        let path = entry.path();
        if path.is_dir() {
            if let Ok(path) = visit_dirs(&path, containerdid) {
                return Ok(path);
            }
        }
    }
    bail!("nothing found");
}

/// return any pid part of this cgroup
pub fn get_cgroup_pid(cgroup: &Path) -> Result<libc::pid_t> {
    let path = cgroup.join("cgroup.procs");
    let bytes = try_with!(fs::read(&path), "cannot read {:?}", &path);
    let pids = try_with!(
        String::from_utf8(bytes),
        "kernel does not respond with valid encoding"
    );
    let pids = pids.splitn(2, '\n').collect::<Vec<&str>>()[0]; // first line
    let pid: u64 = try_with!(u64::from_str(pids), "cannot parse pid ({:?})", pids);
    Ok(pid as libc::pid_t)
}
