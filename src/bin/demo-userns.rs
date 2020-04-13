use caps;
use clap::{crate_version, App, Arg};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::{getegid, geteuid};
use std::process;
use std::thread;
use std::time::Duration;

const STACK_SIZE: usize = 1024 * 1024;

fn child_func(do_loop: bool) -> isize {
    loop {
        println!("Child: eUID: {}, eGID: {}", geteuid(), getegid());
        println!(
            "Child: Capabilities (effective): {:#?}",
            caps::read(None, caps::CapSet::Effective).unwrap()
        );

        if !do_loop {
            break;
        }

        thread::sleep(Duration::from_secs(5));
    }

    0
}

fn main() {
    let matches = App::new("demo-userns")
        .version(crate_version!())
        .arg(
            Arg::with_name("loop")
                .help("print capabilities in a loop")
                .short("l")
                .long("loop"),
        )
        .get_matches();

    let do_loop = matches.is_present("loop");

    let mut child_stack: [u8; STACK_SIZE] = [0; STACK_SIZE];

    println!("Parent: eUID: {}, eGID: {}", geteuid(), getegid());
    println!(
        "Parent: Capabilities (effective): {:#?}",
        caps::read(None, caps::CapSet::Effective).unwrap()
    );

    let pid = clone(
        Box::new(|| child_func(do_loop)),
        &mut child_stack,
        CloneFlags::CLONE_NEWUSER,
        Some(Signal::SIGCHLD as i32),
    )
    .expect("clone() failed");

    waitpid(pid, None).expect("waitpid() failed");
    process::exit(0);
}
