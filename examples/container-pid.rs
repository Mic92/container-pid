use container_pid::{lookup_container_pid, lookup_container_type};
use std::env;
use std::process::exit;

pub(crate) fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("USAGE: {} container-name [container-type]", args[0]);
        exit(1);
    }
    let types = if args.len() >= 3 {
        match lookup_container_type(&args[2]) {
            None => {
                eprintln!("unsupported container type: {}", args[2]);
                exit(1);
            }
            Some(c) => vec![c],
        }
    } else {
        vec![]
    };
    let name = &args[1];
    match lookup_container_pid(&name, &types) {
        Ok(pid) => {
            println!("{}", pid);
        }
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    }
}
