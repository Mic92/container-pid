use std::fs;
use simple_error::{bail, try_with};

use crate::Container;
use crate::result::Result;

#[derive(Clone, Debug)]
pub struct Command {}

impl Container for Command {
    fn lookup(&self, container_id: &str) -> Result<libc::pid_t> {
        let needle = container_id.as_bytes();
        let dir = try_with!(fs::read_dir("/proc"), "failed to read /proc directory");
        let own_pid = std::process::id() as libc::pid_t;

        for entry in dir {
            let entry = try_with!(entry, "error while reading /proc");
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

        bail!("No command found that matches {}", container_id)
    }
    fn check_required_tools(&self) -> Result<()> {
        Ok(())
    }
}
