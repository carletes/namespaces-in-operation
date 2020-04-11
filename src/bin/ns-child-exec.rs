use clap::{crate_version, App, Arg};
use nix::sched::{clone, CloneFlags};

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
}
