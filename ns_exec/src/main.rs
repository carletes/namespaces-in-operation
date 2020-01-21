use std::env;
use std::ffi::{CStr, CString};
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::process;

extern crate nix;

use nix::sched::{setns, CloneFlags};
use nix::unistd::execvp;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} /proc/<pid>/ns/<ns-file> <cmd> [<arg> ..]",
            args[0]
        );
        process::exit(1);
    }

    // Get descriptor for namespace.
    let fd = OpenOptions::new()
        .read(true)
        .open(&args[1])
        .expect("open() failed");

    // Join that namespace.
    setns(fd.as_raw_fd(), CloneFlags::empty()).expect("setns() failed");

    // Execute a command in namespace
    let args_exec_owned: Vec<CString> = args
        .iter()
        .skip(2)
        .map(|a| CString::new(a.as_bytes()).unwrap())
        .collect();
    let args_exec: Vec<&CStr> = args_exec_owned.iter().map(CString::as_c_str).collect();

    execvp(&args_exec[0], &args_exec).expect("exec() failed");
}
