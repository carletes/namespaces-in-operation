# PID namespaces

We now turn to look at PID namespaces. The global resource isolated by
PID namespaces is the process ID number space. This means that processes
in different PID namespaces can have the same process ID. PID namespaces
are used to implement containers that can be migrated between host systems
while keeping the same process IDs for the processes inside the container.

As with processes on a traditional Linux (or UNIX) system, the process IDs
within a PID namespace are unique, and are assigned sequentially starting
with PID 1. Likewise, as on a traditional Linux system, PID 1 &mdash; the
`init` process &mdash; is special: it is the first process created within
the namespace, and it performs certain management tasks within the namespace.


## First investigations

A new PID namespace is created by calling [`clone()`][clone] with the
`CLONE_NEWPID` flag. We'll show a simple example program that creates a
new PID namespace using `clone()` and use that program to map out a few of
the basic concepts of PID namespaces. The complete source of the program
(`pidns-init-sleep.rs`) can be found [here][pidns-init-sleep]. As with the
previous chapter, in the interests of brevity, we omit some error-checking
and other ancilliary code that is present in the full versions of the
example program when discussing it in the body of the article.

The main program creates a new PID namespace using `clone()`, and displays
the PID of the resulting child:

```rust,ignore
let pid = clone(
    Box::new(|| child_func(mount_point)),
    &mut child_stack,
    CloneFlags::CLONE_NEWPID,
    Some(Signal::SIGCHLD as i32),
)
.expect("clone() failed");
println!("PID returned by clone(): {}", pid);
```

The new child process starts execution in `child_func()`, which receives
the last argument of the clone() call (`mount_poit`) as its argument. The
purpose of this argument will become clear later.

The `child_func()` function displays the process ID and parent process ID
of the child created by `clone()` and concludes by executing the standard
`sleep` program:

```rust,ignore
println!("child_func(): PID:  {}", process::id());
println!("child_func(): PPID: {}", parent_id());

// ...

let args_owned: Vec<CString> =
    vec![CString::new("sleep").unwrap(), CString::new("1000").unwrap()];
let args_exec: Vec<&CStr> = args_owned.iter().map(CString::as_c_str).collect();
execvp(&args_exec[0], &args_exec).expect("execvp() failed");
```

The main virtue of executing the `sleep` program is that it provides us
with an easy way of distinguishing the child process from the parent in
process listings.

When we run this program, the first lines of output are as follows:

    $ su         # Need privilege to create a PID namespace
    Password:
    # ./pidns_init_sleep /proc2
    PID returned by clone(): 27656
    childFunc(): PID  = 1
    childFunc(): PPID = 0
    Mounting procfs at /proc2

The first two lines line of output from `pidns-init-sleep` show the PID of
the child process from the perspective of two different PID namespaces: the
namespace of the caller of `clone()` and the namespace in which the child
resides. In other words, the child process has two PIDs: 27656 in the parent
namespace, and 1 in the new PID namespace created by the `clone()` call.

The next line of output shows the parent process ID of the child, within
the context of the PID namespace in which the child resides (i.e.,
the value returned by `getppid()`). The parent PID is 0, demonstrating
a small quirk in the operation of PID namespaces. As we detail below,
PID namespaces form a hierarchy: a process can "see" only those processes
contained in its own PID namespace and in the child namespaces nested below
that PID namespace. Because the parent of the child created by `clone()`
is in a different namespace, the child cannot "see" the parent; therefore,
`getppid()` reports the parent PID as being zero.

For an explanation of the last line of output from `pidns-init-sleep`,
we need to return to a piece of code that we skipped when discussing the
implementation of the `child_func()` function.


## `/proc/PID` and PID namespaces

Each process on a Linux system has a `/proc/PID` directory that contains
pseudo-files describing the process. This scheme translates directly into the
PID namespaces model. Within a PID namespace, the `/proc/PID` directories
show information only about processes within that PID namespace or one of
its descendant namespaces.

However, in order to make the `/proc/PID` directories that correspond to
a PID namespace visible, the `proc` filesystem ("procfs" for short) needs
to be mounted from within that PID namespace. From a shell running inside
the PID namespace (perhaps invoked via the `system()` library function),
we can do this using a `mount` command of the following form:

```text
# mount -t proc proc /mount_point
```

Alternatively, a procfs can be mounted using the `mount()` system call,
as is done inside our program's `child_func()` function:

```rust,ignore
// Create directory for mount point.
create_dir_all(mount_point).expect("create_dir() failed");

const NONE: Option<&'static [u8]> = None;
mount(
    Some("proc"),
    mount_point.as_bytes(),
    Some("proc"),
    MsFlags::empty(),
    NONE,
)
.expect("mount() failed");
println!("Mounted procfs at {}", mount_point);
```

The `mount_point` variable is initialized from the string supplied as the
command-line argument when invoking `pidns-init-sleep`.

In our example shell session running `pidns-init-sleep` above, we mounted
the new procfs at `/proc2`. In real world usage, the procfs would (if it is
required) usually be mounted at the usual location, `/proc`, using either
of the techniques that we describe in a moment. However, mounting the
procfs at `/proc2` during our demonstration provides an easy way to avoid
creating problems for the rest of the processes on the system: since those
processes are in the same mount namespace as our test program, changing
the filesystem mounted at `/proc` would confuse the rest of the system by
making the `/proc/PID` directories for the root PID namespace invisible.

Thus, in our shell session the procfs mounted at `/proc` will show the PID
subdirectories for the processes visible from the parent PID namespace,
while the procfs mounted at `/proc2` will show the PID subdirectories for
processes that reside in the child PID namespace. In passing, it's worth
mentioning that although the processes in the child PID namespace will be
able to see the PID directories exposed by the `/proc` mount point, those
PIDs will not be meaningful for the processes in the child PID namespace,
since system calls made by those processes interpret PIDs in the context
of the PID namespace in which they reside.

Having a procfs mounted at the traditional `/proc` mount point is necessary
if we want various tools such as `ps` to work correctly inside the child PID
namespace, because those tools rely on information found at `/proc`. There
are two ways to achieve this without affecting the `/proc` mount point used
by parent PID namespace. First, if the child process is created using the
`CLONE_NEWNS` flag, then the child will be in a different mount namespace
from the rest of the system. In this case, mounting the new procfs at
`/proc` would not cause any problems. Alternatively, instead of employing
the `CLONE_NEWNS` flag, the child could change its root directory with
`chroot()` and mount a procfs at `/proc`.

Let's return to the shell session running `pidns-init-sleep`. We stop
the program and use `ps` to examine some details of the parent and child
processes within the context of the parent namespace:

```text
^Z                          Stop the program, placing in background
[1]+  Stopped                 ./pidns_init_sleep /proc2
# ps -C sleep -C pidns_init_sleep -o "pid ppid stat cmd"
PID  PPID STAT CMD
27655 27090 T    ./pidns_init_sleep /proc2
27656 27655 S    sleep 600
```

The "PPID" value (27655) in the last line of output above shows that
the parent of the process executing sleep is the process executing
`pidns-init-sleep`.

By using the `readlink` command to display the (differing) contents of the
`/proc/PID/ns/pid` symbolic links (explained in the previous chapter),
we can see that the two processes are in separate PID namespaces:

```text
# readlink /proc/27655/ns/pid
pid:[4026531836]
# readlink /proc/27656/ns/pid
pid:[4026532412]
```

At this point, we can also use our newly mounted procfs to obtain information
about processes in the new PID namespace, from the perspective of that
namespace. To begin with, we can obtain a list of PIDs in the namespace
using the following command:

```text
# ls -d /proc2/[1-9]*
/proc2/1
```

As can be seen, the PID namespace contains just one process, whose PID
(inside the namespace) is 1. We can also use the `/proc/PID/status` file
as a different method of obtaining some of the same information about that
process that we already saw earlier in the shell session:

```text
# cat /proc2/1/status | egrep '^(Name|PP*id)'
Name:   sleep
Pid:    1
PPid:   0
```

The `PPid` field in the file is 0, matching the fact that `getppid()`
reports that the parent process ID for the child is 0.


## Nested PID namespaces

As noted earlier, PID namespaces are hierarchically nested in parent-child
relationships. Within a PID namespace, it is possible to see all other
processes in the same namespace, as well as all processes that are members
of descendant namespaces. Here, "see" means being able to make system calls
that operate on specific PIDs (e.g., using `kill()` to send a signal to
process). Processes in a child PID namespace cannot see processes that exist
(only) in the parent PID namespace (or further removed ancestor namespaces).

A process will have one PID in each of the layers of the PID namespace
hierarchy starting from the PID namespace in which it resides through to
the root PID namespace. Calls to `getpid()` always report the PID associated
with the namespace in which the process resides.

We can use the program shown [here][multi-pidns] (`multi-pidns.rs`) to show
that a process has different PIDs in each of the namespaces in which it
is visible. In the interests of brevity, we will simply explain what the
program does, rather than walking though its code.

The program recursively creates a series of child process in nested PID
namespaces. The command-line flag `--levels` specified when invoking the
program determines how many children and PID namespaces to create:

```text
$ cargo build
$ sudo ./target/debug/multi-pidns --levels 5
```

In addition to creating a new child process, each recursive step mounts
a procfs filesystem at a uniquely named mount point. At the end of the
recursion, the last child executes the `sleep` program. The above command
line yields the following output:

```text
child_func(4): Mounted procfs at /tmp/proc4
child_func(3): Mounted procfs at /tmp/proc3
child_func(2): Mounted procfs at /tmp/proc2
child_func(1): Mounted procfs at /tmp/proc1
child_func(0): Mounted procfs at /tmp/proc0
child_func(0): Final child sleeping ..
```

Looking at the PIDs in each procfs, we see that each successive procfs
"level" contains fewer PIDs, reflecting the fact that each PID namespace
shows only the processes that are members of that PID namespace or its
descendant namespaces:

```text
^Z
[1]+  Stopped                 sudo ./target/debug/multi-pidns --levels 5
$ ls -d /tmp/proc4/[1-9]*
/tmp/proc4/1  /tmp/proc4/2  /tmp/proc4/3  /tmp/proc4/4  /tmp/proc4/5
$ ls -d /tmp/proc3/[1-9]*
/tmp/proc3/1  /tmp/proc3/2  /tmp/proc3/3  /tmp/proc3/4
$ ls -d /tmp/proc2/[1-9]*
/tmp/proc2/1  /tmp/proc2/2  /tmp/proc2/3
$ ls -d /tmp/proc1/[1-9]*
/tmp/proc1/1  /tmp/proc1/2
$ ls -d /tmp/proc0/[1-9]*
/tmp/proc0/1
```

A suitable `grep` command allows us to see the PID of the process at the
tail end of the recursion (i.e., the process executing sleep in the most
deeply nested namespace) in all of the namespaces where it is visible:

```text
$ grep -H 'Name:.*sleep' /tmp/proc?/[1-9]*/status
/tmp/proc0/1/status:Name:       sleep
/tmp/proc1/2/status:Name:       sleep
/tmp/proc2/3/status:Name:       sleep
/tmp/proc3/4/status:Name:       sleep
/tmp/proc4/5/status:Name:       sleep
```

In other words, in the most deeply nested PID namespace (`/tmp/proc0`),
the process executing sleep has the PID 1, and in the topmost PID namespace
created (`/tmp/proc4`), that process has the PID 5.

If you run the test programs shown in this chapter, it's worth mentioning that
they will leave behind mount points and mount directories. After terminating
the programs, shell commands such as the following should suffice to clean
things up:

```text
$ sudo umount /tmp/proc?
```

[clone]: http://man7.org/linux/man-pages/man2/clone.2.html
[multi-pidns]: ../src/bin/multi-pidns.rs
[pidns-init-sleep]: ../src/bin/pidns-init-sleep.rs
