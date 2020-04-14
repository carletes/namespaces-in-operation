use nix::mount::{mount, umount, MsFlags};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::execvp;
use std::env;
use std::ffi::{CStr, CString};
use std::fs::{create_dir_all, remove_dir};
use std::os::unix::process::parent_id;
use std::process;

fn child_func(mount_point: &str) -> isize {
    println!("child_func(): PID:  {}", process::id());
    println!("child_func(): PPID: {}", parent_id());

    create_dir_all(mount_point).expect("create_dir() failed");

    const NONE: Option<&'static [u8]> = None;
    mount(
        Some("proc"),
        mount_point.as_bytes(),
        Some("proc"),
        MsFlags::empty(),
        NONE,
    )
    .expect("mount() failed");
    println!("Mounted procfs at {}", mount_point);

    let args_owned: Vec<CString> = vec![
        CString::new("sleep").unwrap(),
        CString::new("1000").unwrap(),
    ];
    let args_exec: Vec<&CStr> = args_owned.iter().map(CString::as_c_str).collect();
    execvp(&args_exec[0], &args_exec).expect("execvp() failed");

    0
}

const STACK_SIZE: usize = 1024 * 1024;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <proc-mount-point>", args[0]);
        process::exit(1);
    }
    let mount_point = &args[1];

    let mut child_stack: [u8; STACK_SIZE] = [0; STACK_SIZE];
    let pid = clone(
        Box::new(|| child_func(mount_point)),
        &mut child_stack,
        CloneFlags::CLONE_NEWPID,
        Some(Signal::SIGCHLD as i32),
    )
    .expect("clone() failed");
    println!("PID returned by clone(): {}", pid);

    waitpid(pid, None).expect("waitpid() failed");

    umount(mount_point.as_bytes()).expect("umount() failed");
    println!("Unounted procfs at {}", mount_point);
    remove_dir(mount_point).expect("remove_dir() failed");
    process::exit(0);
}
