use anyhow::{bail, Context};
use std::fs;

use crate::result::Result;
use crate::Container;

#[derive(Clone, Debug)]
pub(crate) struct Command {}

impl Container for Command {
    fn lookup(&self, container_id: &str) -> Result<libc::pid_t> {
        let needle = container_id.as_bytes();
        let dir =
            fs::read_dir("/proc").context("failed to read /proc directory - is procfs mounted?")?;
        let own_pid = std::process::id() as libc::pid_t;

        for entry in dir {
            let entry = entry.context("failed to read entry in /proc")?;
            let cmdline = entry.path().join("cmdline");
            let pid = match entry.file_name().to_string_lossy().parse::<libc::pid_t>() {
                Ok(pid) => pid,
                _ => {
                    continue;
                }
            };
            if pid == own_pid {
                continue;
            }

            // ignore error if process exits before we can read it
            if let Ok(mut arguments) = fs::read(cmdline.clone()) {
                // treat all arguments as one large string
                for byte in arguments.iter_mut() {
                    if *byte == b'\0' {
                        *byte = b' ';
                    }
                }
                if arguments
                    .windows(needle.len())
                    .any(|window| window == needle)
                {
                    return Ok(pid);
                }
            }
        }

        bail!(
            "no process found with command line matching '{}'",
            container_id
        )
    }
    fn check_required_tools(&self) -> Result<()> {
        Ok(())
    }
}
