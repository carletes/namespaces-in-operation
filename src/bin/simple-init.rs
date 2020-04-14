use clap::{crate_version, App, Arg};
use libc::c_int;
use nix::errno::Errno;
use nix::sys::signal::{sigaction, signal, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag};
use nix::unistd::{execvp, fork, getpgrp, pause, setpgid, tcsetpgrp, ForkResult, Pid};
use nix::Error;
use std::ffi::{CStr, CString};
use std::io::{self, Write};
use std::process;
use wordexp::{wordexp, Wordexp};

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
            Ok(status) => {
                if let Some(pid) = status.pid() {
                    unsafe {
                        if VERBOSE {
                            println!("\tinit: SIGCHLD handler: PID {} terminated", pid);
                        }
                    }
                }
                continue;
            }
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

    let verbose = matches.is_present("verbose");

    unsafe {
        VERBOSE = verbose;
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
    }

    if verbose {
        println!("\tinit: my PID is {}", process::id());
    }

    // Performing terminal operations while not being the foreground
    // process group for the terminal generates a SIGTTOU that stops the
    // process.  However our init "shell" needs to be able to perform
    // such operations (just like a normal shell), so we ignore that
    // signal, which allows the operations to proceed successfully.
    unsafe {
        signal(Signal::SIGTTOU, SigHandler::SigIgn).expect("signal(SIGTTOU, SIG_IGN) failed");
    }

    // Become leader of a new process group and make that process group the
    // foreground process group for the terminal.
    setpgid(Pid::from_raw(0), Pid::from_raw(0)).expect("setpgid() failed");
    tcsetpgrp(0, getpgrp()).expect("tcsetpgrp() failed");

    loop {
        print!("init$ ");
        io::stdout().flush().unwrap();

        // Read a shell command; exit on end of file .
        let mut cmd = String::new();
        if io::stdin().read_line(&mut cmd).unwrap() == 0 {
            if verbose {
                print!("\n\tinit: Exiting");
                io::stdout().flush().unwrap();
            }
            println!();
            process::exit(0);
        }

        // Strip trailing newline and other whitespace characters.
        let cmd = cmd.trim();

        // Ignore empty commands.
        if cmd.is_empty() {
            continue;
        }

        match wordexp(cmd, Wordexp::new(0), 0) {
            Err(e) => {
                println!("\tinit: Error processing command line: {}", e);
                continue;
            }
            Ok(wexp) => {
                let args_owned: Vec<CString> = wexp
                    .map(|a| CString::new(a).expect("error creating C string"))
                    .collect();

                // Create child process.
                match fork().expect("fork() failed") {
                    ForkResult::Child => {
                        // Make child the leader of a new process group and
                        // make that process group the foreground process group
                        // for the terminal.
                        setpgid(Pid::from_raw(0), Pid::from_raw(0)).expect("setpgid() failed");
                        tcsetpgrp(0, getpgrp()).expect("tcsetpgrp() failed");

                        // Child executes shell command and terminates.
                        let args_exec: Vec<&CStr> =
                            args_owned.iter().map(CString::as_c_str).collect();
                        execvp(&args_exec[0], &args_exec).expect("execvp() failed");
                    }
                    ForkResult::Parent { child } => {
                        if verbose {
                            println!("\tinit: Created child {}", child);
                        }

                        // Will be interrupted by signal handler
                        pause();

                        // After child changes state, ensure that the 'init'
                        // program is the foreground process group for the
                        // terminal.
                        tcsetpgrp(0, getpgrp()).expect("tcsetpgrp() failed");
                    }
                }
            }
        }
    }
}
