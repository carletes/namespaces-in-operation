use nix::unistd::{fork, getppid, ForkResult, Pid};
use std::process;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    match fork().expect("fork() failed") {
        ForkResult::Parent { child } => {
            let pid = process::id();
            println!("Parent (PID: {}) created child with PID {}", pid, child);
            println!("Parent (PID: {}, PPID:{}) terminating", pid, getppid());
            process::exit(0);
        }
        ForkResult::Child => {
            let pid_of_init = Pid::from_raw(1);
            loop {
                if getppid() == pid_of_init {
                    // Our parent is `init` now.
                    break;
                }
                sleep(Duration::from_millis(100));
            }

            let pid = process::id();
            println!(
                "Child (PID: {}) now an orphan (parent PID: {})",
                pid,
                getppid()
            );

            sleep(Duration::from_secs(1));

            println!("Child (PID: {}) terminating", pid);
            process::exit(0);
        }
    }
}
