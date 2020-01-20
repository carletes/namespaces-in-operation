use std::env;
use std::ffi::CString;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::process;

extern crate nix;

use nix::sched::{setns, CloneFlags};
use nix::unistd::execvp;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} /proc/<pid>/ns/<ns-file> <cmd>", args[0]);
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
    let prog = CString::new(args[2].as_bytes()).expect("CString::new(prog) failed");
    execvp(&prog, &[&prog]).expect("exec() failed");
}
