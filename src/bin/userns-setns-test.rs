use nix::sched::{clone, setns, CloneFlags};
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use std::env;
use std::fs::{read_link, OpenOptions};
use std::os::unix::io::{AsRawFd, RawFd};
use std::process;
use std::thread;
use std::time::Duration;

const STACK_SIZE: usize = 1024 * 1024;

// Try to join the user namespace identified by the file descriptor `fd`.
// `pname` is a per-process string that the caller may use to distinguish
// information messages displayed by this function.
fn test_setns(pname: &str, fd: RawFd) {
    // Display caller's user namespace ID.
    let path = read_link("/proc/self/ns/user").expect("readlink() failed");
    println!(
        "{}: readlink(\"/proc/self/ns/user\"): {}",
        pname,
        path.display()
    );

    // Attempt to join the user namespace specified by `fd`.
    match setns(fd, CloneFlags::CLONE_NEWUSER) {
        Ok(()) => println!("{}: setns() succeeded", pname),
        Err(err) => println!("!{}: setns() failed: {}", pname, err),
    }
}

fn child_func(fd: RawFd) -> isize {
    // Avoid intermingling with parent's output.
    thread::sleep(Duration::from_micros(100000));

    // Test whether `setns()` is possible from the child user namespace.
    //
    test_setns("child", fd);
    0
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} /proc/PID/ns/user", args[0]);
        process::exit(1);
    }

    // Open user namespace file specified on command line.
    let fd = OpenOptions::new()
        .read(true)
        .open(&args[1])
        .expect("open() failed")
        .as_raw_fd();

    // Create child process in new user namespace.
    let mut child_stack: [u8; STACK_SIZE] = [0; STACK_SIZE];
    let pid = clone(
        Box::new(|| child_func(fd)),
        &mut child_stack,
        CloneFlags::CLONE_NEWUSER,
        Some(Signal::SIGCHLD as i32),
    )
    .expect("clone() failed");

    waitpid(pid, None).expect("waitpid() failed");

    process::exit(0);
}
