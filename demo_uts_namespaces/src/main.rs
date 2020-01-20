extern crate nix;

use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::sys::utsname::uname;
use nix::sys::wait::waitpid;
use nix::unistd::sethostname;
use std::env;
use std::process;
use std::thread;

const STACK_LENGTH: usize = 1024 * 1024;

fn child_func(hostname: &String) -> isize {
    sethostname(hostname).expect("sethostname() failed");
    let uts = uname();
    println!("uts.nodename in child: {}", uts.nodename());
    thread::sleep_ms(100 * 1000);
    return 0;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <child-hostname>", args[0]);
        process::exit(1);
    }

    let mut child_stack: [u8; STACK_LENGTH] = [0; STACK_LENGTH];

    let pid = clone(
        Box::new(|| child_func(&args[1])),
        &mut child_stack,
        CloneFlags::CLONE_NEWUTS,
        Some(Signal::SIGCHLD as i32),
    )
    .expect("clone() failed");
    println!("PID of child created by clone(): {}", pid);

    thread::sleep_ms(1 * 1000);

    let uts = uname();
    println!("uts.nodename in parent: {}", uts.nodename());

    waitpid(pid, None).expect("waitpid() failed");
    println!("child has terminated");

    process::exit(0);
}
