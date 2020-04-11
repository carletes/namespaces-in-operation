use clap::{crate_version, App, Arg};
use libc::c_int;
use nix::errno::Errno;
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag};
use nix::Error;
use std::process;

static mut VERBOSE: bool = false;

extern "C" fn child_handler(_: c_int) {
    let mut flags = WaitPidFlag::empty();

    // WUNTRACED and WCONTINUED allow waitpid() to catch stopped and continued
    // children (in addition to terminated children).
    flags.set(WaitPidFlag::WNOHANG, true);
    flags.set(WaitPidFlag::WUNTRACED, true);
    flags.set(WaitPidFlag::WCONTINUED, true);

    loop {
        match waitpid(None, Some(flags)) {
            Ok(_) => continue,
            Err(Error::Sys(Errno::ECHILD)) => {
                // No more children
                break;
            }
            Err(err) => {
                // Unexpected error.
                panic!("waitpid() failed: {}", err);
            }
        }
    }
}

fn main() {
    let matches = App::new("ns-child-exec")
        .version(crate_version!())
        .arg(
            Arg::with_name("verbose")
                .help("verbose operation")
                .short("v")
                .long("verbose"),
        )
        .get_matches();

    if matches.is_present("verbose") {
        unsafe {
            VERBOSE = true;
        }
    }

    let mut sa_flags = SaFlags::empty();
    sa_flags.set(SaFlags::SA_RESTART, true);
    sa_flags.set(SaFlags::SA_NOCLDSTOP, true);

    let sa = SigAction::new(
        SigHandler::Handler(child_handler),
        sa_flags,
        SigSet::empty(),
    );

    unsafe {
        sigaction(Signal::SIGCHLD, &sa).expect("sigaction(SIGCHLD) failed");

        if VERBOSE {
            println!("\tinit: my PID is {}", process::id());
        }
    }

    // Performing terminal operations while not being the foreground
    // process group for the terminal generates a SIGTTOU that stops the
    // process.  However our init "shell" needs to be able to perform
    // such operations (just like a normal shell), so we ignore that
    // signal, which allows the operations to proceed successfully.
}
