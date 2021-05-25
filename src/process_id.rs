use libc::pid_t;
use std::fs;
use std::io::ErrorKind;
use simple_error::{try_with, bail};
use std::ffi::OsString;
use std::path::PathBuf;
use std::env;

use crate::Container;
use crate::result::Result;

#[derive(Clone, Debug)]
pub struct ProcessId {}

/// TODO make this configureable?
fn get_path() -> PathBuf {
    PathBuf::from(&env::var_os("CNTR_PROC").unwrap_or_else(|| OsString::from("/proc")))
}

impl Container for ProcessId {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let pid = match container_id.parse::<pid_t>() {
            Err(e) => try_with!(Err(e), "not a valid pid: `{}`", container_id),
            Ok(v) => v,
        };

        match fs::metadata(get_path().join(pid.to_string())) {
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    bail!("no process with pid {} found", pid)
                } else {
                    try_with!(Err(e), "could not lookup process {}", pid)
                }
            }
            Ok(_) => Ok(pid),
        }
    }
    fn check_required_tools(&self) -> Result<()> {
        Ok(())
    }
}
