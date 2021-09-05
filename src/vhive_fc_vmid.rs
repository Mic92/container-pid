//! This module takes a firecracker vm id and searches a vhive setup for the open file-descriptors
//! of relevant processes for one belonging to that vm id.

use crate::result::Result;
use crate::Container;
use simple_error::{bail, try_with};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct VhiveFcVmid {}

impl Container for VhiveFcVmid {
    fn lookup(&self, container_id: &str) -> Result<libc::pid_t> {
        let pid = try_with!(
            find_fc_pid(&container_id),
            "cannot find pid for firecracker vmID {}",
            container_id
        );
        Ok(pid)
    }

    fn check_required_tools(&self) -> Result<()> {
        Ok(())
    }
}

/// search which process has known_file open
pub fn find_fc_pid(vmid: &str) -> Result<libc::pid_t> {
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
