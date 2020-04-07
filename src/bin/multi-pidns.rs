use clap::{crate_version, value_t, App, Arg};
use nix::mount::{mount, MsFlags};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::execvp;
use std::ffi::{CStr, CString};
use std::fs::create_dir_all;
use std::process;

const NONE: Option<&'static [u8]> = None;
const STACK_SIZE: usize = 1024 * 1024;

fn child_func(level: u8, first_call: bool) -> isize {
    if !first_call {
        // Unless this is the first recursive call to  `child_func()`
        // (i.e., we were invoked from `main`), mount a procfs or the current
        // PID namespace.

        let mount_point = format!("/tmp/proc{}", level);
        create_dir_all(mount_point.clone()).expect("create_dir() failed");
        mount(
            Some("proc"),
            mount_point.as_bytes(),
            Some("proc"),
            MsFlags::empty(),
            NONE,
        )
        .expect("mount() failed");
        println!("child_func({}): Mounted procfs at {}", level, mount_point);
    }

    if level > 0 {
        // Recursively invoke  `child_func()` to create another child in a
        // nested PID namespace.
        let mut child_stack: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let pid = clone(
            Box::new(|| child_func(level - 1, false)),
            &mut child_stack,
            CloneFlags::CLONE_NEWPID,
            Some(Signal::SIGCHLD as i32),
        )
        .expect("clone() failed");

        waitpid(pid, None).expect("waitpid() failed");
    } else {
        // Tail end of recursion: execute  `sleep(1)`.
        println!("child_func({}): Final child sleeping ...", level);

        let args_owned: Vec<CString> = vec![
            CString::new("sleep").unwrap(),
            CString::new("1000").unwrap(),
        ];
        let args_exec: Vec<&CStr> = args_owned.iter().map(CString::as_c_str).collect();
        execvp(&args_exec[0], &args_exec).expect("execvp() failed");
    }

    0
}

fn main() {
    let matches = App::new("multi-pidns")
        .version(crate_version!())
        .arg(
            Arg::with_name("levels")
                .help("Number of nested PID namespace levels")
                .default_value("5")
                .long("levels")
                .short("l"),
        )
        .get_matches();

    let levels = value_t!(matches, "levels", u8).unwrap();
    child_func(levels, true);
    process::exit(0);
}
