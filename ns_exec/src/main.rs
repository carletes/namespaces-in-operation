use std::env;
use std::ffi::CString;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::process;

extern crate nix;

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

    let fd = File::open(&args[1]).expect("open() failed").as_raw_fd();

    // if (setns(fd, 0) == -1)         /* Join that namespace */
    //     errExit("setns");

    let prog_c = CString::new(args[0].as_bytes()).expect("CString::new(prog) failed");
    let prog_args_c = CString::new(args[2..]).expect("CString::new(prog_args) failed");
    execvp(&prog_c, &[&prog_args_c]).expect("exec() failed");
}
