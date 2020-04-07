# The namespaces API

A namespace wraps a global system resource in an abstraction that makes it
appear to the processes within the namespace that they have their own isolated
instance of the resource. Namespaces are used for a variety of purposes,
with the most notable being the implementation of containers, a technique
for lightweight virtualization. This chapter looks at the namespaces API
in some detail and shows the API in action in a number of example programs.

The namespace API consists of three system calls &mdash; `clone()`,
`unshare()`, and `setns()` &mdash; and a number of `/proc` files. In this
chapter, we'll look at all of these system calls and some of the `/proc`
files. In order to specify a namespace type on which to operate, the three
system calls make use of the following `CLONE_NEW*` constants: `CLONE_NEWIPC`,
`CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWPID`, `CLONE_NEWUSER`, and
`CLONE_NEWUTS`.


## Creating a child in a new namespace: `clone()`

One way of creating a namespace is via the use of [`clone()`][clone], a system
call that creates a new process. For our purposes, we will be calling the
`clone()` system call using the Rust [nix::sched::clone()][nix-sched-clone]
function:

```rust,ignore
pub fn clone(
    cb: CloneCb,
    stack: &mut [u8],
    flags: CloneFlags,
    signal: Option<c_int>
) -> Result<pid_t>
```

Essentially, `clone()` is a more general version of the traditional UNIX
`fork()` system call whose functionality can be controlled via the `flags`
argument. In all, there are more than twenty different `CLONE_*` flags that
control various aspects of the operation of `clone()`, including whether
the parent and child process share resources such as virtual memory, open
file descriptors, and signal dispositions. If one of the `CLONE_NEW*`
bits is specified in the call, then a new namespace of the corresponding
type is created, and the new process is made a member of that namespace;
multiple `CLONE_NEW*` bits can be specified in `flags`.

Our first example program ([`demo-uts-namespace.rs`][demo-uts-namespace])
uses `nix::sched::clone()` with the `CLONE_NEWUTS` flag to create a UTS
namespace. UTS namespaces isolate two system identifiers &mdash; the hostname
and the NIS domain name &mdash; that are set using the `sethostname()`
and `setdomainname()` system calls and returned by the `uname()` system
call.  You can find the full source of the program if you follow the link
above. Below, we'll focus on just some of the key pieces of the program.

The example program takes one command-line argument. When run, it creates
a child that executes in a new UTS namespace. Inside that namespace, the
child changes the hostname to the string given as the program's command-line
argument.

The first significant piece of the main program is the `clone()` call that
creates the child process:

```rust,ignore
let pid = clone(
    Box::new(|| child_func(&args[1])),
    &mut child_stack,
    CloneFlags::CLONE_NEWUTS,
    Some(Signal::SIGCHLD as i32),
)
.expect("clone() failed");
println!("PID of child created by clone(): {}", pid);
```

The new child will begin execution in the user-defined function
`child_func()`; that function will receive a reference to the program's
command-line argument as its argument. Since `CLONE_NEWUTS` is specified
as part of the flags argument, the child will execute in a newly created
UTS namespace.

The main program then sleeps for a moment. This is a (crude) way of giving
the child time to change the hostname in its UTS namespace. The program
then uses [`nix::sys::utsname::uname()`][nix-uname] to retrieve the host
name in the parent's UTS namespace, and displays that hostname:

```rust,ignore
thread::sleep(Duration::from_secs(1));

let uts = uname();
println!("uts.nodename in parent: {}", uts.nodename());
```

Meanwhile, the `child_func()` function executed by the child created by
`clone()` first changes the hostname to the value supplied in its argument,
and then retrieves and displays the modified hostname:

```rust,ignore
sethostname(hostname).expect("sethostname() failed");

let uts = uname();
println!("uts.nodename in child: {}", uts.nodename());
```

Before terminating, the child sleeps for a while. This has the effect of
keeping the child's UTS namespace open, and gives us a chance to conduct
some of the experiments that we show later.

Running the program demonstrates that the parent and child processes have
independent UTS namespaces. You will need to run it as `root`, since root
privileges are needed to create UTS namespaces:

```text
$ cargo build
$ uname -n
rilke
$ sudo ./target/debug/demo-uts-namespaces bizarro
PID of child created by clone(): 6549
uts.nodename in child: bizarro
uts.nodename in parent: rilke
```

As with most other namespaces (user namespaces are the exception), creating
a UTS namespace requires privilege (specifically, `CAP_SYS_ADMIN`). This is
necessary to avoid scenarios where set-user-ID applications could be fooled
into doing the wrong thing because the system has an unexpected hostname.

Another possibility is that a set-user-ID application might be using the
hostname as part of the name of a lock file. If an unprivileged user could
run the application in a UTS namespace with an arbitrary hostname, this
would open the application to various attacks. Most simply, this would
nullify the effect of the lock file, triggering misbehavior in instances
of the application that run in different UTS namespaces. Alternatively, a
malicious user could run a set-user-ID application in a UTS namespace with
a hostname that causes creation of the lock file to overwrite an important
file. (Hostname strings can contain arbitrary characters, including slashes.)


## The `/proc/PID/ns` files

Each process has a `/proc/PID/ns` directory that contains one file for
each type of namespace. Starting in Linux 3.8, each of these files is a
special symbolic link that provides a kind of handle for performing certain
operations on the associated namespace for the process.

```text
$ ls -l /proc/$$/ns         # $$ is replaced by shell's PID
total 0
lrwxrwxrwx 1 carlos carlos 0 Apr  9 17:08 cgroup -> 'cgroup:[4026531835]'
lrwxrwxrwx 1 carlos carlos 0 Apr  9 17:08 ipc -> 'ipc:[4026531839]'
lrwxrwxrwx 1 carlos carlos 0 Apr  9 17:08 mnt -> 'mnt:[4026531840]'
lrwxrwxrwx 1 carlos carlos 0 Apr  9 17:08 net -> 'net:[4026532001]'
lrwxrwxrwx 1 carlos carlos 0 Apr  9 17:08 pid -> 'pid:[4026531836]'
lrwxrwxrwx 1 carlos carlos 0 Apr  9 17:08 pid_for_children -> 'pid:[4026531836]'
lrwxrwxrwx 1 carlos carlos 0 Apr  9 17:08 user -> 'user:[4026531837]'
lrwxrwxrwx 1 carlos carlos 0 Apr  9 17:08 uts -> 'uts:[4026531838]'
```

One use of these symbolic links is to discover whether two processes are
in the same namespace. The kernel does some magic to ensure that if two
processes are in the same namespace, then the inode numbers reported for the
corresponding symbolic links in `/proc/PID/ns` will be the same. The inode
numbers can be obtained using the `stat()` system call (in the `st_ino`
field of the returned structure).

However, the kernel also constructs each of the `/proc/PID/ns` symbolic
links so that it points to a name consisting of a string that identifies
the namespace type, followed by the inode number. We can examine this name
using either the `ls -l` or the `readlink` command.

Let's return to the shell session above where we ran the `demo-uts-namespaces`
program. Looking at the `/proc/PID/ns` symbolic links for the parent and
child process provides an alternative method of checking whether the two
processes are in the same or different UTS namespaces:

```text
^Z                                # Stop parent and child
[1]+  Stopped                 sudo ./target/debug/demo-uts-namespaces bizarro
$ ps auxww | grep demo-uts-namespaces
root      6542  0.0  0.0   8908  3544 pts/3    T    16:45   0:00 sudo ./target/debug/demo-uts-namespaces bizarro
root      6548  0.0  0.0   3556  2536 pts/3    T    16:45   0:00 ./target/debug/demo-uts-namespaces bizarro
root      6549  0.0  0.0   3556  1148 pts/3    T    16:45   0:00 ./target/debug/demo-uts-namespaces bizarro
$ sudo readlink /proc/6548/ns/uts
uts:[4026531838]
$ sudo readlink /proc/6549/ns/uts
uts:[4026532628]
```

As can be seen, the content of the `/proc/PID/ns/uts` symbolic links differs,
indicating that the two processes are in different UTS namespaces.

The `/proc/PID/ns` symbolic links also serve other purposes. If we open
one of these files, then the namespace will continue to exist as long as
the file descriptor remains open, even if all processes in the namespace
terminate. The same effect can also be obtained by bind mounting one of
the symbolic links to another location in the file system:

```text
$ sudo touch /root/uts                   # Create mount point.
$ sudo mount --bind /proc/6549/ns/uts /root/uts
```

Before Linux 3.8, the files in `/proc/PID/ns` were hard links rather than
special symbolic links of the form described above. In addition, only the
`ipc`, `net`, and `uts` files were present.


## Joining an existing namespace: `setns()`

Keeping a namespace open when it contains no processes is of course only
useful if we intend to later add processes to it. That is the task of the
[`setns()`][setns] system call, which allows the calling process to join
an existing namespace:

```text
int setns(int fd, int nstype);
```

This system call is available in Rust through the
[`nix::sched::setns()`][nix-sched-setns] function:

```rust,ignore
pub fn setns(fd: RawFd, nstype: CloneFlags) -> Result<()>
```

More precisely, `setns()` disassociates the calling process from one instance
of a particular namespace type and reassociates the process with another
instance of the same namespace type.

The `fd` argument specifies the namespace to join; it is a file descriptor
that refers to one of the symbolic links in a `/proc/PID/ns` directory. That
file descriptor can be obtained either by opening one of those symbolic
links directly or by opening a file that was bind mounted to one of the links.

The `nstype` argument allows the caller to check the type of namespace
that `fd` refers to. If this argument is specified as zero, no check is
performed. This can be useful if the caller already knows the namespace type,
or does not care about the type. The example program that we discuss in a
moment (`ns-exec.rs`) falls into the latter category: it is designed to
work with any namespace type. Specifying `nstype` instead as one of the
`CLONE_NEW*` constants causes the kernel to verify that `fd` is a file
descriptor for the corresponding namespace type. This can be useful if,
for example, the caller was passed the file descriptor via a UNIX domain
socket and needs to verify what type of namespace it refers to.

Using `setns()` and `execve()` (or one of the other `exec()` functions)
allows us to construct a simple but useful tool: a program that joins a
specified namespace and then executes a command in that namespace.

Our program ([ns-exec.rs][ns-exec]) takes two or more command-line
arguments. The first argument is the pathname of a `/proc/PID/ns/*` symbolic
link (or a file that is bind mounted to one of those symbolic links). The
remaining arguments are the name of a program to be executed inside the
namespace that corresponds to that symbolic link and optional command-line
arguments to be given to that program. The key steps in the program are
the following:

```rust,ignore
// Get descriptor for namespace.
let fd = OpenOptions::new()
    .read(true)
    .open(&args[1])
    .expect("open() failed");

// Join that namespace.
setns(fd.as_raw_fd(), CloneFlags::empty()).expect("setns() failed");

// Execute a command in namespace.
let args_exec_owned: Vec<CString> = args
    .iter()
    .skip(2)
    .map(|a| CString::new(a.as_bytes()).unwrap())
    .collect();
let args_exec: Vec<&CStr> = args_exec_owned.iter().map(CString::as_c_str).collect();
```

An interesting program to execute inside a namespace is, of course, a
shell. We can use the bind mount for the UTS namespace that we created
earlier in conjunction with the `ns-exec` program to execute a shell in
the new UTS namespace created by our invocation of `demo-uts-namespaces`:

```text

$ sudo ./target/debug/ns-exec /root/uts /bin/bash
```

We can then verify that the shell is in the same UTS namespace as the child
process created by `demo-uts-namespaces`, both by inspecting the hostname
and by comparing the inode numbers of the `/proc/PID/ns/uts` files:

```text
# hostname
bizarro
# readlink /proc/$$/ns/uts
uts:[4026532628]
# readlink /proc/6549/ns/uts
uts:[4026532628]
```

In earlier kernel versions, it was not possible to use `setns()` to join
mount, PID, and user namespaces, but, starting with Linux 3.8, `setns()`
now supports joining all namespace types.


## Leaving a namespace: unshare()

The final system call in the namespaces API is [`unshare()`][unshare]:

```text
int unshare(int flags);
```

We will call `unsare()` using the Rust function
[`nix::sched::unshare()`][nix-sched-unshare]:

```rust,ignore
pub fn unshare(flags: CloneFlags) -> Result<()>
```

The `unshare()` system call provides functionality similar to `clone()`, but
operates on the calling process: it creates the new namespaces specified by
the `CLONE_NEW*` bits in its `flags` argument and makes the caller a member
of the namespaces. (As with `clone()`, `unshare()` provides functionality
beyond working with namespaces that we'll ignore here.) The main purpose of
`unshare()` is to isolate namespace (and other) side effects without having
to create a new process or thread (as is done by `clone()`).

Leaving aside the other effects of the `clone()` system call, a call of
the form:

```rust,ignore
let pid = clone(
    // ..
    CloneFlags::CLONE_NEWXXX,
    // ..
)
```

is roughly equivalent, in namespace terms, to the sequence:

```rust,ignore
match fork() {
   Ok(ForkResult::Child) => unshare(CloneFlags::CLONE_NEWXXX),
   // ..
}
```

One use of the `unshare()` system call is in the implementation of the
`unshare` command, which allows the user to execute a command in a separate
namespace from the shell. The general form of this command is:

    unshare [options] program [arguments]

The options are command-line flags that specify the namespaces to unshare
before executing program with the specified arguments.

The key steps in the implementation of the unshare command are straightforward:

```rust,ignore
// Code to initialize `flags` according to command-line options omitted.

unshare(flags).expect("unshare() failed");

// Now execute the given command line.

execvp(&args_exec[0], &args_exec).expect("exec() failed");
```

A simple implementation of the `unshare` command ([`unshare.rs`][unshare-rs])
can be found by following the link.

In the following shell session, we use our `unshare.rs` program to execute
a shell in a separate mount namespace. Mount namespaces isolate the set of
filesystem mount points seen by a group of processes, allowing processes
in different mount namespaces to have different views of the filesystem
hierarchy.

```text
$ echo $$                             # Show PID of shell
13031
$ cat /proc/13031/mounts | grep mq    # Show one of the mounts in namespace
mqueue /dev/mqueue mqueue rw,relatime 0 0
$ readlink /proc/$$/ns/mnt            # Show mount namespace ID
mnt:[4026531840]
$ sudo ./target/debug/unshare -m /bin/bash
# readlink /proc/$$/ns/mnt
mnt:[4026532628]
```

Comparing the output of the two `readlink` commands shows that the two
shells are in separate mount namespaces. Altering the set of mount points
in one of the namespaces and checking whether that change is visible in the
other namespace provides another way of demonstrating that the two programs
are in separate namespaces:

```text
# umount /dev/mqueue                  # Remove a mount point in this shell
# cat /proc/$$/mounts | grep mq       # Verify that mount point is gone
# cat /proc/8490/mounts | grep mq     # Is it still present in the other namespace?
mqueue /dev/mqueue mqueue rw,relatime 0 0
```

As can be seen from the output of the last two commands, the `/dev/mqueue`
mount point has disappeared in one mount namespace, but continues to exist
in the other.


## Concluding remarks

In this chapter we've looked at the fundamental pieces of the namespace
API and how they are employed together. In the following chapters we'll
look in more depth at some other namespaces, in particular, the PID and
user namespaces; user namespaces open up a range of new possibilities for
applications to use kernel interfaces that were formerly restricted to
privileged applications.


[clone]: http://man7.org/linux/man-pages/man2/clone.2.html
[containers]: https://lwn.net/Articles/524952/
[demo-uts-namespace]: ../src/bin/demo-uts-namespaces.rs
[nix-sched-clone]: https://docs.rs/nix/0.17.0/nix/sched/fn.clone.html
[nix-sched-setns]: https://docs.rs/nix/0.17.0/nix/sched/fn.setns.html
[nix-sched-unshare]: https://docs.rs/nix/0.17.0/nix/sched/fn.unshare.html
[nix-uname]: https://docs.rs/nix/0.17.0/nix/sys/utsname/fn.uname.html
[ns-exec]: ../src/bin/ns-exec.rs
[setns]: http://man7.org/linux/man-pages/man2/setns.2.html
[unshare]: http://man7.org/linux/man-pages/man2/unshare.2.html
[unshare-rs]: ../src/bin/unshare.rs
