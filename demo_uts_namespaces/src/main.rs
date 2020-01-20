extern crate nix;

use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use std::env;
use std::process;

const STACK_LENGTH: usize = 1024 * 1024;

// static int              /* Start function for cloned child */
//     childFunc(void *arg)
// {
//     struct utsname uts;

//     /* Change hostname in UTS namespace of child */
//     if (sethostname(arg, strlen(arg)) == -1)
//         errExit("sethostname");

//     /* Retrieve and display hostname */
//     if (uname(&uts) == -1)
//         errExit("uname");
//     printf("uts.nodename in child:  %s\n", uts.nodename);

//     /* Keep the namespace open for a while, by sleeping.
//     This allows some experimentation--for example, another
//     process might join the namespace. */
//     sleep(100);

//     return 0;           /* Terminates child */
// }

fn child_func(hostname: &String) -> isize {
    return 0;
}

fn main() {
    // pid_t child_pid;
    // struct utsname uts;

    // /* Create a child that has its own UTS namespace;
    //    the child commences execution in childFunc() */
    // child_pid = clone(childFunc,
    //                 child_stack + STACK_SIZE,   /* Points to start of
    //                                                downwardly growing stack */
    //                 CLONE_NEWUTS | SIGCHLD, argv[1]);
    // if (child_pid == -1)
    //     errExit("clone");
    // printf("PID of child created by clone() is %ld\n", (long) child_pid);

    // /* Parent falls through to here */
    // sleep(1);           /* Give child time to change its hostname */
    // /* Display the hostname in parent's UTS namespace. This will be
    //    different from the hostname in child's UTS namespace. */
    // if (uname(&uts) == -1)
    //     errExit("uname");
    // printf("uts.nodename in parent: %s\n", uts.nodename);

    // if (waitpid(child_pid, NULL, 0) == -1)      /* Wait for child */
    //     errExit("waitpid");
    // printf("child has terminated\n");

    // exit(EXIT_SUCCESS);
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <child-hostname>", args[0]);
        process::exit(1);
    }

    let mut child_stack: [u8; STACK_LENGTH] = [0; STACK_LENGTH];

    clone(
        Box::new(|| child_func(&args[1])),
        &mut child_stack,
        CloneFlags::CLONE_NEWUTS,
        Some(Signal::SIGCHLD as i32),
    );

    process::exit(0);
}
