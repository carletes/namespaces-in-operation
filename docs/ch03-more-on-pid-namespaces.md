# More on PID namespaces

One use of PID namespaces is to implement a package of processes (a container)
that behaves like a self-contained Linux system. A key part of a traditional
system &mdash; and likewise a PID namespace container &mdash; is the `init``
process. Thus, we'll look at the special role of the `init` process in a PID
namespace, and note one or two areas where it differs from the traditional
`init` process. In addition, we'll look at some other details of the
namespaces API as it applies to PID namespaces.


## The PID namespace init process

The first process created inside a PID namespace gets a process ID of 1
within the namespace. This process has a similar role to the `init` process
on traditional Linux systems. In particular, the `init` process can perform
initializations required for the PID namespace as whole (e.g., perhaps
starting other processes that should be a standard part of the namespace)
and becomes the parent for processes in the namespace that become orphaned.

In order to explain the operation of PID namespaces, we'll make use
of a few purpose-built example programs. The first of these programs,
[`ns-child-exec.rs`][ns-child-exec], has the following command-line syntax:

```text
ns-child-exec [options] command [arguments]
```

The `ns-child-exec` program uses the `clone()` system call to create a
child process; the child then executes the given command with the optional
arguments. The main purpose of the options is to specify new namespaces
that should be created as part of the `clone()` call. For example, the
`-p` option causes the child to be created in a new PID namespace, as in
the following example:

```text
$ su                  # Need privilege to create a PID namespace
Password:
# ./ns_child_exec -p sh -c 'echo $$'
1
```

That command line creates a child in a new PID namespace to execute a shell
echo command that displays the shell's PID. With a PID of 1, the shell was
the `init` process for the PID namespace that (briefly) existed while the
shell was running.

Our next example program, [`simple-init.rs`][simple-init], is a program
that we'll execute as the `init`process of a PID namespace. This program is
designed to allow us to demonstrate some features of PID namespaces and the
`init` process.

The `simple-init` program performs the two main functions of `init`. One
of these functions is "system initialization". Most `init` systems
are more complex programs that take a table-driven approach to system
initialization. Our (much simpler) `simple-init` program provides a simple
shell facility that allows the user to manually execute any shell commands
that might be needed to initialize the namespace; this approach also allows
us to freely execute shell commands in order to conduct experiments in the
namespace. The other function performed by `simple-init` is to reap the
status of its terminated children using `waitpid()`.

Thus, for example, we can use the `ns-child-exec` program in conjunction with
`simple-init` to fire up an `init` process that runs in a new PID namespace:

```text
# ./ns_child_exec -p ./simple_init
init$
```

The `init$ prompt` indicates that the `simple-init` program is ready to
read and execute a shell command.

We'll now use the two programs we've presented so far in conjunction with
another small program, [`orphan.rs`][orphan], to demonstrate that processes
that become orphaned inside a PID namespace are adopted by the PID namespace
`init` process, rather than the system-wide `init` process.

The orphan program performs a `fork()` to create a child process. The parent
process then exits while the child continues to run; when the parent exits,
the child becomes an orphan. The child executes a loop that continues until
it becomes an orphan (i.e., `getppid()` returns 1); once the child becomes
an orphan, it terminates. The parent and the child print messages so that we
can see when the two processes terminate and when the child becomes an orphan.

In order to see what that our `simple-init` program reaps the orphaned
child process, we'll employ that program's `-v` option, which causes it
to produce verbose messages about the children that it creates and the
terminated children whose status it reaps:

```text
# ./ns_child_exec -p ./simple_init -v
        init: my PID is 1
init$ ./orphan
        init: created child 2
Parent (PID=2) created child with PID 3
Parent (PID=2; PPID=1) terminating
        init: SIGCHLD handler: PID 2 terminated
init$                   # simple_init prompt interleaved with output from child
Child  (PID=3) now an orphan (parent PID=1)
Child  (PID=3) terminating
        init: SIGCHLD handler: PID 3 terminated
```

In the above output, the indented messages prefixed with `init:` are printed
by the `simple-init` program's verbose mode. All of the other messages (other
than the `init$` prompts) are produced by the orphan program. From the output,
we can see that the child process (PID 3) becomes an orphan when its parent
(PID 2) terminates. At that point, the child is adopted by the PID namespace
`init` process (PID 1), which reaps the child when it terminates.


## Signals and the init process

The traditional Linux `init` process is treated specially with respect to
signals. The only signals that can be delivered to `init` are those for
which the process has established a signal handler; all other signals
are ignored. This prevents the `init` process &mdash; whose presence
is essential for the stable operation of the system &mdash; from being
accidentally killed, even by the superuser.

PID namespaces implement some analogous behavior for the namespace-specific
`init` process. Other processes in the namespace (even privileged processes)
can send only those signals for which the `init` process has established a
handler. This prevents members of the namespace from inadvertently killing a
process that has an essential role in the namespace. Note, however, that (as
for the traditional `init` process) the kernel can still generate signals
for the PID namespace `init` process in all of the usual circumstances
(e.g., hardware exceptions, terminal-generated signals such as `SIGTTOU`,
and expiration of a timer).

Signals can also (subject to the usual permission checks) be sent to the
PID namespace `init` process by processes in ancestor PID namespaces. Again,
only the signals for which the `init` process has established a handler can
be sent, with two exceptions: `SIGKILL` and `SIGSTOP`. When a process in an
ancestor PID namespace sends these two signals to the `init` process, they
are forcibly delivered (and can't be caught). The `SIGSTOP` signal stops the
`init` process; `SIGKILL` terminates it. Since the `init` process is essential
to the functioning of the PID namespace, if the `init` process is terminated
by `SIGKILL` (or it terminates for any other reason), the kernel terminates
all other processes in the namespace by sending them a `SIGKILL` signal.

Normally, a PID namespace will also be destroyed when its `init` process
terminates. However, there is an unusual corner case: the namespace won't be
destroyed as long as a `/proc/PID/ns/pid` file for one of the processes in
that namespaces is bind mounted or held open. However, it is not possible
to create new processes in the namespace (via `setns()` plus `fork()`):
the lack of an `init` process is detected during the `fork()` call, which
fails with an `ENOMEM error` (the traditional error indicating that a PID
cannot be allocated). In other words, the PID namespace continues to exist,
but is no longer usable.


## Mounting a procfs filesystem (revisited)

In the previous article in this series, the `/proc` filesystems (procfs)
for the PID namespaces were mounted at various locations other than the
traditional `/proc` mount point. This allowed us to use shell commands
to look at the contents of the `/proc/PID` directories that corresponded
to each of the new PID namespace while at the same time using the `ps`
command to look at the processes visible in the root PID namespace.

However, tools such as `ps` rely on the contents of the procfs mounted at
`/proc` to obtain the information that they require. Therefore, if we want
`ps` to operate correctly inside a PID namespace, we need to mount a procfs
for that namespace. Since the `simple-init` program permits us to execute
shell commands, we can perform this task from the command line, using the
`mount` command:

```text
# ./ns_child_exec -p -m ./simple_init
init$ mount -t proc proc /proc
init$ ps a
  PID TTY      STAT   TIME COMMAND
    1 pts/8    S      0:00 ./simple_init
    3 pts/8    R+     0:00 ps a
```

The `ps a` command lists all processes accessible via `/proc`. In this
case, we see only two processes, reflecting the fact that there are only
two processes running in the namespace.

When running the `ns-child-exec` command above, we employed that program's
`-m` option, which places the child that it creates (i.e., the process
running `simple-init`) inside a separate mount namespace. As a consequence,
the `mount` command does not affect the `/proc` mount seen by processes
outside the namespace.


## unshare() and setns()

In the first chapter, we described two system calls that are part of the
namespaces API: `unshare()` and `setns()`. Since Linux 3.8, these system
calls can be employed with PID namespaces, but they have some idiosyncrasies
when used with those namespaces.

Specifying the `CLONE_NEWPID` flag in a call to `unshare()` creates a new
PID namespace, but does not place the caller in the new namespace. Rather,
any children created by the caller will be placed in the new namespace;
the first such child will become the `init` process for the namespace.

The `setns()` system call now supports PID namespaces:

```text
    setns(fd, 0);   /* Second argument can be CLONE_NEWPID to force a
                       check that 'fd' refers to a PID namespace */
```

The `fd` argument is a file descriptor that identifies a PID namespace that
is a descendant of the PID namespace of the caller; that file descriptor is
obtained by opening the `/proc/PID/ns/pid` file for one of the processes
in the target namespace. As with `unshare()`, `setns()` does not move the
caller to the PID namespace; instead, children that are subsequently created
by the caller will be placed in the namespace.

We can use an enhanced version of the [`ns-exec.rs`][ns-exec] program
that we presented in the first chapter in this series to demonstrate some
aspects of using `setns()` with PID namespaces that appear surprising until
we understand what is going on. The new program, [`ns-run.rs`][ns-run],
has the following syntax:

```text
ns-run [-f] [-n /proc/PID/ns/FILE]... command [arguments]
```

The program uses `setns()` to join the namespaces specified by the
`/proc/PID/ns` files contained within `-n` options. It then goes on to execute
the given command with optional arguments. If the `-f` option is specified, it
uses `fork()` to create a child process that is used to execute the command.

Suppose that, in one terminal window, we fire up our `simple-init` program
in a new PID namespace in the usual manner, with verbose logging so that
we are informed when it reaps child processes:

```text
# ./ns_child_exec -p ./simple_init -v
        init: my PID is 1
init$
```

Then we switch to a second terminal window where we use the `ns-run` program
to execute our orphan program. This will have the effect of creating two
processes in the PID namespace governed by `simple-init`:

```text
# ps -C sleep -C simple_init
  PID TTY          TIME CMD
9147 pts/8    00:00:00 simple_init
# ./ns_run -f -n /proc/9147/ns/pid ./orphan
Parent (PID=2) created child with PID 3
Parent (PID=2; PPID=0) terminating
#
Child  (PID=3) now an orphan (parent PID=1)
Child  (PID=3) terminating
```

Looking at the output from the "Parent" process (PID 2) created when the
`orphan` program is executed, we see that its parent process ID is 0. This
reflects the fact that the process that started the `orphan` process
(`ns-run`) is in a different namespace &mdash; one whose members are
invisible to the "Parent" process. As already noted in the previous article,
`getppid()` returns 0 in this case.


The following diagram shows the relationships of the various processes before
the `orphan` "Parent" process terminates. The arrows indicate parent-child
relationships between processes.

```text
xxx Missing image!
```

Returning to the window running the `simple-init` program, we see the
following output:

```text
init: SIGCHLD handler: PID 3 terminated
```

The "Child" process (PID 3) created by the `orphan` program was reaped by
`simple-init` but the "Parent" process (PID 2) was not. This is because
the "Parent" process was reaped by its parent (`ns-run`) in a different
namespace. The following diagram shows the processes and their relationships
after the `orphan` "Parent" process has terminated and before the "Child"
terminates.

```text
xxx Missing image!
```

It's worth emphasizing that `setns()` and `unshare()` treat PID namespaces
specially. For other types of namespaces, these system calls do change the
namespace of the caller. The reason that these system calls do not change the
PID namespace of the calling process is because becoming a member of another
PID namespace would cause the process's idea of its own PID to change, since
`getpid()` reports the process's PID with respect to the PID namespace in
which the process resides. Many user-space programs and libraries rely on
the assumption that a process's PID (as reported by `getpid()`) is constant
(in fact, the GNU C library `getpid()` wrapper function caches the PID); those
programs would break if a process's PID changed. To put things another way: a
process's PID namespace membership is determined when the process is created,
and (unlike other types of namespace membership) cannot be changed thereafter.


[ns-child-exec]: ../src/bin/ns-child-exec.rs
[simple-init]: ../src/bin/simple-init.rs
[orphan]: ../src/bin/orphan.rs
[ns-exec]: ../src/bin/ns-exec.rs
[ns-run]: ../src/bin/ns-run.rs