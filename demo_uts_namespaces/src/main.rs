use std::env;
use std::process;
use std::thread;
use std::time::Duration;

extern crate nix;

use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::sys::utsname::uname;
use nix::sys::wait::waitpid;
use nix::unistd::sethostname;

const STACK_LENGTH: usize = 1024 * 1024;

fn child_func(hostname: &String) -> isize {
    // Change hostname in UTS namespace of child
    sethostname(hostname).expect("sethostname() failed");

    let uts = uname();
    println!("uts.nodename in child: {}", uts.nodename());

    // Keep the namespace open for a while, by sleeping. This allows some
    // experimentation --- for example, another process might join the
    // namespace.
    thread::sleep(Duration::from_secs(100));

    // Terminates child.
    return 0;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <child-hostname>", args[0]);
        process::exit(1);
    }

    let mut child_stack: [u8; STACK_LENGTH] = [0; STACK_LENGTH];

    // Create a child that has its own UTS namespace; the child commences
    // execution in `child_func` above.
    let pid = clone(
        Box::new(|| child_func(&args[1])),
        &mut child_stack,
        CloneFlags::CLONE_NEWUTS,
        Some(Signal::SIGCHLD as i32),
    )
    .expect("clone() failed");
    println!("PID of child created by clone(): {}", pid);

    // Give child time to change its hostname.
    thread::sleep(Duration::from_secs(1));

    // Display the hostname in parent's UTS namespace. This will be different
    // from the hostname in child's UTS namespace.
    let uts = uname();
    println!("uts.nodename in parent: {}", uts.nodename());

    waitpid(pid, None).expect("waitpid() failed");
    println!("child has terminated");

    process::exit(0);
}
