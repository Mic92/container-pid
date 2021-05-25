use libc::pid_t;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::BufReader;
use std::process::Command;
use simple_error::{try_with, bail};

use crate::cmd;
use crate::Container;
use crate::result::Result;

#[derive(Clone, Debug)]
pub struct Rkt {}

fn find_child_processes(parent_pid: &str) -> Result<pid_t> {
    let dir = try_with!(fs::read_dir("/proc"), "failed to read /proc directory");

    for entry in dir {
        let entry = try_with!(entry, "error while reading /proc");
        let status_path = entry.path().join("status");
        if let Ok(file) = File::open(status_path.clone()) {
            // ignore if process exits before we can read it
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = try_with!(line, "could not read {}", status_path.display());
                let columns: Vec<&str> = line.splitn(2, '\t').collect();
                assert!(columns.len() == 2);
                if columns[0] == "PPid:" && columns[1] == parent_pid {
                    let pid = try_with!(
                        entry.file_name().to_string_lossy().parse::<pid_t>(),
                        "read invalid pid from proc: '{}'",
                        columns[1]
                    );
                    return Ok(pid);
                }
            }
        }
    }

    bail!("no child process found for pid {}", parent_pid)
}

impl Container for Rkt {
    fn lookup(&self, container_id: &str) -> Result<pid_t> {
        let command = format!("rkt status {}", container_id);
        let output = try_with!(
            Command::new("rkt").args(&["status", container_id]).output(),
            "Running '{}' failed",
            command
        );

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Failed to list containers. '{}' exited with {}: {}",
                command,
                output.status,
                stderr.trim_end()
            );
        }

        let lines = output.stdout.split(|&c| c == b'\n');
        let mut rows = lines.map(|line| {
            let cols: Vec<&[u8]> = line.splitn(2, |&c| c == b'=').collect();
            cols
        });
        if let Some(pid_row) = rows.find(|cols| cols[0] == b"pid") {
            assert!(pid_row.len() == 2);
            let ppid = String::from_utf8_lossy(pid_row[1]);
            Ok(try_with!(
                find_child_processes(&ppid),
                "could not find container process belonging to rkt container '{}'",
                container_id
            ))
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            bail!(
                "expected to find `pid=` field in output of '{}', got: {}",
                command, stdout
            )
        }
    }
    fn check_required_tools(&self) -> Result<()> {
        if cmd::which("rkt").is_some() {
            Ok(())
        } else {
            bail!("rkt not found")
        }
    }
}
