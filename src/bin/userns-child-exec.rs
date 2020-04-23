use clap::{crate_version, App, Arg};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::{close, execvp, pipe, read};
use std::ffi::{CStr, CString};
use std::os::unix::io::RawFd;
use std::process;

// Update the mapping file 'map_file', with the value provided in
// 'mapping', a string that defines a UID or GID mapping. A UID or
// GID mapping consists of one or more newline-delimited records
// of the form:
//
// ID_inside-ns    ID-outside-ns   length
//
// Requiring the user to supply a string that contains newlines is
// of course inconvenient for command-line use. Thus, we permit the
// use of commas to delimit records in this string, and replace them
// with newlines before writing the string to the file.
//
// static void
// update_map(char *mapping, char *map_file)
// {
// int fd, j;
// size_t map_len;     [> Length of 'mapping' <]
//
// Replace commas in mapping string with newlines
//
// map_len = strlen(mapping);
// for (j = 0; j < map_len; j++)
// if (mapping[j] == ',')
// mapping[j] = '\n';
//
// fd = open(map_file, O_RDWR);
// if (fd == -1) {
// fprintf(stderr, "open %s: %s\n", map_file, strerror(errno));
// exit(EXIT_FAILURE);
// }
//
// if (write(fd, mapping, map_len) != map_len) {
// fprintf(stderr, "write %s: %s\n", map_file, strerror(errno));
// exit(EXIT_FAILURE);
// }
//
// close(fd);
// }

fn child_func(args: &[&CStr], reader: RawFd, writer: RawFd) -> isize {
    // Wait until the parent has updated the UID and GID mappings. See
    // the comment in `main()`. We wait for end of file on a pipe that will
    // be closed by the parent process once it has updated the mappings.

    // Close our descriptor for the write end of the pipe so that we see EOF
    // when parent closes its descriptor.
    close(writer).expect("close() failed in child");

    let mut buf: [u8; 1] = [0; 1];
    read(reader, &mut buf).expect("read() from pipe failed in child");

    execvp(&args[0], &args).expect("exec() failed");
    0
}

const STACK_LENGTH: usize = 1024 * 1024;

fn main() {
    let matches = App::new("userns-child-exec")
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
            Arg::with_name("uid-map")
                .help("UID map string for user namespace")
                .short("M")
                .takes_value(true)
                .value_name("MAP")
                .long("uid-map"),
        )
        .arg(
            Arg::with_name("gid-map")
                .help("GID map string for user namespace")
                .short("G")
                .takes_value(true)
                .value_name("MAP")
                .long("gid-map"),
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

    let verbose = matches.is_present("verbose");
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

    // We use a pipe to synchronize the parent and child, in order to
    // ensure that the parent sets the UID and GID maps before the child
    // calls execve(). This ensures that the child maintains its
    // capabilities during the execve() in the common case where we
    // want to map the child's effective user ID to 0 in the new user
    // namespace. Without this synchronization, the child would lose
    // its capabilities if it performed an execve() with nonzero
    // user IDs (see the capabilities(7) man page for details of the
    // transformation of a process's capabilities during execve()).

    let (reader, writer) = pipe().expect("pipe() failed");

    let pid = clone(
        Box::new(|| child_func(&args_exec, reader, writer)),
        &mut child_stack,
        flags,
        Some(Signal::SIGCHLD as i32),
    )
    .expect("clone() failed");

    if verbose {
        println!(
            "userns-child-exec: PID of child created by clone is {}",
            pid
        );
    }

    // Update the UID and GID maps in the child.
    //
    // if (uid_map != NULL) {
    // snprintf(map_path, PATH_MAX, "/proc/%ld/uid_map",
    // (long) child_pid);
    // update_map(uid_map, map_path);
    // }
    // if (gid_map != NULL) {
    // snprintf(map_path, PATH_MAX, "/proc/%ld/gid_map",
    // (long) child_pid);
    // update_map(gid_map, map_path);
    // }

    // Close the write end of the pipe, to signal to the child that we
    // have updated the UID and GID maps.
    close(writer).expect("close() failed");

    // Parent process: Wait for child.
    waitpid(pid, None).expect("waitpid() failed");

    if verbose {
        println!("userns-child-exec: Terminating");
    }
    process::exit(0);
}
