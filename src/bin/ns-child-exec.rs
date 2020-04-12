use clap::{crate_version, App, Arg};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::execvp;
use std::ffi::{CStr, CString};
use std::process;

fn child_func(args: &[&CStr]) -> isize {
    execvp(&args[0], &args).expect("exec() failed");
    0
}

const STACK_LENGTH: usize = 1024 * 1024;

fn main() {
    let matches = App::new("ns-child-exec")
        .version(crate_version!())
        .arg(
            Arg::with_name("ipc")
                .help("unshare IPC namespace")
                .short("i")
                .long("ipc"),
        )
        .arg(
            Arg::with_name("mount")
                .help("unshare mount namespace")
                .short("m")
                .long("mount"),
        )
        .arg(
            Arg::with_name("net")
                .help("unshare network namespace")
                .short("n")
                .long("net"),
        )
        .arg(
            Arg::with_name("pid")
                .help("unshare PID namespace")
                .short("p")
                .long("pid"),
        )
        .arg(
            Arg::with_name("uts")
                .help("unshare UTS namespace")
                .short("u")
                .long("uts"),
        )
        .arg(
            Arg::with_name("user")
                .help("unshare user namespace")
                .short("U")
                .long("user"),
        )
        .arg(
            Arg::with_name("verbose")
                .help("verbose operation")
                .short("v")
                .long("verbose"),
        )
        .arg(Arg::with_name("cmd").index(1).required(true))
        .arg(Arg::with_name("arg").multiple(true))
        .get_matches();

    let mut flags = CloneFlags::empty();
    let mut verbose = false;

    if matches.is_present("ipc") {
        flags.set(CloneFlags::CLONE_NEWIPC, true)
    }
    if matches.is_present("mount") {
        flags.set(CloneFlags::CLONE_NEWNS, true)
    }
    if matches.is_present("net") {
        flags.set(CloneFlags::CLONE_NEWNET, true)
    }
    if matches.is_present("pid") {
        flags.set(CloneFlags::CLONE_NEWPID, true)
    }
    if matches.is_present("uts") {
        flags.set(CloneFlags::CLONE_NEWUTS, true)
    }
    if matches.is_present("user") {
        flags.set(CloneFlags::CLONE_NEWUSER, true)
    }
    if matches.is_present("verbose") {
        verbose = true;
    }

    let mut child_stack: [u8; STACK_LENGTH] = [0; STACK_LENGTH];

    let cmd = matches.value_of("cmd").unwrap();
    let mut args_exec_owned: Vec<CString> = vec![CString::new(cmd).unwrap()];
    if matches.is_present("arg") {
        matches
            .values_of("arg")
            .unwrap()
            .for_each(|a| args_exec_owned.push(CString::new(a).unwrap()));
    }
    let args_exec: Vec<&CStr> = args_exec_owned.iter().map(CString::as_c_str).collect();

    let pid = clone(
        Box::new(|| child_func(&args_exec)),
        &mut child_stack,
        flags,
        Some(Signal::SIGCHLD as i32),
    )
    .expect("clone() failed");

    if verbose {
        println!("ns-child-exec: PID of child created by clone is {}", pid);
    }

    // Parent process: Wait for child.
    waitpid(pid, None).expect("waitpid() failed");

    if verbose {
        println!("ns-child-exec: Terminating");
    }
    process::exit(0);
}
