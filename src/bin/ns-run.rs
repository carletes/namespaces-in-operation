use clap::{crate_version, App, Arg};
use nix::sched::{setns, CloneFlags};
use nix::sys::wait::waitpid;
use nix::unistd::{execvp, fork, ForkResult};
use std::ffi::{CStr, CString};
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::process;

fn main() {
    let matches = App::new("ns-run")
        .version(crate_version!())
        .arg(
            Arg::with_name("fork")
                .help("fork before exec")
                .short("f")
                .long("fork"),
        )
        .arg(
            Arg::with_name("ns")
                .help("path to the /proc/PID/ns/<ns> of the namespace to join")
                .short("n")
                .long("ns")
                .required(true)
                .takes_value(true)
                .value_name("PATH"),
        )
        .arg(Arg::with_name("cmd").index(1).required(true))
        .arg(Arg::with_name("arg").multiple(true))
        .get_matches();

    let do_fork = if matches.is_present("fork") {
        true
    } else {
        false
    };

    // Get descriptor for namespace.
    let fd = OpenOptions::new()
        .read(true)
        .open(&matches.value_of("ns").unwrap())
        .expect("open() failed");

    // Join that namespace.
    setns(fd.as_raw_fd(), CloneFlags::empty()).expect("setns() failed");

    if do_fork {
        match fork().expect("fork() failed") {
            ForkResult::Parent { child } => {
                // Wait for child and exit.
                waitpid(child, None).expect("waitpid() failed");
                process::exit(0);
            }
            ForkResult::Child => {
                // Fall through to code below.
            }
        }
    }

    let mut args_owned: Vec<CString> =
        vec![CString::new(matches.value_of("cmd").unwrap()).unwrap()];
    if matches.is_present("arg") {
        matches
            .values_of("arg")
            .unwrap()
            .for_each(|a| args_owned.push(CString::new(a).unwrap()));
    }
    let args_exec: Vec<&CStr> = args_owned.iter().map(CString::as_c_str).collect();
    execvp(&args_exec[0], &args_exec).expect("execvp() failed");
}
