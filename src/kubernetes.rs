use crate::result::Result;
use crate::Container;
use serde_json as json;
use simple_error::{bail, require_with, try_with};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::from_utf8;
use std::str::FromStr;
use std::ffi::OsString;
use crate::cmd;

#[derive(Clone, Debug)]
pub struct Kubernetes {}

impl Container for Kubernetes {
    /// There is many ways to do this:
    ///  - similar to command.rs: a bit looser pattern matching on /proc/$pid/cmdline
    ///  - the following:
    fn lookup(&self, container_id: &str) -> Result<libc::pid_t> {
        let containerdid = get_container_id(&"knative-serving", container_id)?;
        let cgroup = find_cgroup(containerdid).unwrap();
        let bytes = try_with!(fs::read(cgroup.join("cgroup.procs")), "");
        let pids = String::from_utf8(bytes).unwrap();
        let pids = require_with!(
            pids.as_str().strip_suffix("\n"),
            "unexpected: Empty cgroup: {:?}",
            cgroup
        );
        let pid: u64 = try_with!(u64::from_str(pids), "Cannot parse pid ({:?}). Is it more than one?");
        Ok(pid as libc::pid_t)
    }

    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("kubectl").is_some() {
            Ok(())
        } else {
            bail!("kubectl not found")
        }
    }
}

// TODO parse container_id as {{namespace/pod_name}} and assume namespace=default if no / is found.
/// find `containerd://hash` id and return hash
pub fn get_container_id(namespace: &str, pod_name: &str) -> Result<String> {
    if namespace.contains("/") || pod_name.contains("/") {
        bail!(
            "namespace and pod_name must not contain '/': {} {}",
            namespace,
            pod_name
        );
    }

    // kubectl get --raw "/api/v1/namespaces/knative-serving/pods/autoscaler-589958b7b6-l4cb6"
    // equivalent to `kubectl describe -n knative-serving autoscaler-589958b7b6-l4cb6`
    let url = format!("/api/v1/namespaces/{}/pods/{}", namespace, pod_name);
    //println!("url {}", url);
    let result = try_with!(
        Command::new("kubectl")
            .arg("get")
            .arg("--raw")
            .arg(url)
            .output(),
        "kubctl command cannot be spawned"
    );

    if !result.status.success() {
        //println!("stdout: {}", from_utf8(&result.stdout).unwrap());
        //println!();
        let stderr = from_utf8(&result.stderr).unwrap();
        bail!(
            "kubectl get pod request failed (ret code {:?}): {}",
            result.status.code(),
            stderr
        );
    }
    //println!("{}", from_utf8(&result.stdout).unwrap());
    let json: json::Value = try_with!(
        json::from_str(from_utf8(&result.stdout).unwrap()),
        "failed to parse kubectl get pod response"
    );
    // TODO search all possible array fields
    let containerid = require_with!(
        json["status"]["containerStatuses"][0]["containerID"].as_str(),
        "failed to parse kubectl get pod response json"
    );
    let containerid = require_with!(
        containerid.strip_prefix("containerd://"),
        "unexpected/unparsable containerd id"
    );
    //println!("containerid {}", String::from(containerid));
    Ok(String::from(containerid))
}

pub fn find_cgroup(containerdid: String) -> Result<PathBuf> {
    let path = visit_dirs(
        &PathBuf::from("/sys/fs/cgroup"),
        &OsString::from(containerdid),
    )
    .unwrap();
    //println!("path {:?}", path);
    Ok(path)
}

// one possible implementation of walking a directory from
// https://doc.rust-lang.org/std/fs/fn.read_dir.html
fn visit_dirs(dir: &Path, containerdid: &OsString) -> Result<PathBuf> {
    //println!("visit_dirs {:?}", dir);
    //if dir.is_dir() {
    for entry in try_with!(std::fs::read_dir(dir), "asdf") {
        let entry = try_with!(entry, "asdf");
        //println!("visiting {:?}", entry.path());
        if &entry.file_name() == containerdid {
            return Ok(entry.path());
        }
        let path = entry.path();
        if path.is_dir() {
            match visit_dirs(&path, containerdid) {
                Ok(path) => return Ok(path),
                Err(_) => {}
            };
        } else {
            //cb(&entry);
        }
    }
    //}
    bail!("Nothing found");
}
