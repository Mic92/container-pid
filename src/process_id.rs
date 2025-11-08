use anyhow::{bail, Context};
use libc::pid_t;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use crate::result::Result;
use crate::Container;

#[derive(Clone, Debug)]
pub(crate) struct ProcessId {}

/// TODO make this configureable?
fn get_path() -> PathBuf {
    PathBuf::from(&env::var_os("CNTR_PROC").unwrap_or_else(|| OsString::from("/proc")))
}

impl Container for ProcessId {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let pid = container_id
            .parse::<pid_t>()
            .with_context(|| format!("'{}' is not a valid PID (process ID)", container_id))?;

        match fs::metadata(get_path().join(pid.to_string())) {
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    bail!("no process with PID {} found", pid)
                } else {
                    Err(e).with_context(|| format!("failed to lookup process {}", pid))?
                }
            }
            Ok(_) => Ok(pid),
        }
    }
    fn check_required_tools(&self) -> Result<()> {
        Ok(())
    }
}
