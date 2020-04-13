# User namespaces

User namespaces allow per-namespace mappings of user and group IDs. This means
that a process's user and group IDs inside a user namespace can be different
from its IDs outside of the namespace. Most notably, a process can have a
nonzero user ID outside a namespace while at the same time having a user ID
of zero inside the namespace; in other words, the process is unprivileged
for operations outside the user namespace but has root privileges inside
the namespace.

## Creating user namespaces

User namespaces are created by specifying the `CLONE_NEWUSER` flag when
calling `clone()` or `unshare()`. Starting with Linux 3.8 (and unlike the
flags used for creating other types of namespaces), no privilege is required
to create a user namespace. In our examples below, all of the user namespaces
are created using the unprivileged user ID 1000.

To begin investigating user namespaces, we'll make use of a small program,
[`demo-userns.rs`][demo-userns], that creates a child in a new user
namespace. The child simply displays its effective user and group IDs as
well as its effective [capabilities][capabilities]. Running this program
as an unprivileged user produces the following result:

```text
$ cargo run --bin demo-userns
Parent: eUID: 1000, eGID: 1000
Parent: Capabilities (effective): {}
Child: eUID: 65534, eGID: 65534
Child: Capabilities (effective): {
    CAP_SETUID,
    CAP_CHOWN,
    CAP_AUDIT_READ,
    CAP_NET_ADMIN,
    CAP_SYS_RAWIO,
    CAP_SETFCAP,
    CAP_LINUX_IMMUTABLE,
    CAP_SYS_BOOT,
    CAP_SYS_MODULE,
    CAP_SYS_ADMIN,
    CAP_NET_BROADCAST,
    CAP_SYS_TTY_CONFIG,
    CAP_MKNOD,
    CAP_IPC_LOCK,
    CAP_MAC_ADMIN,
    CAP_LEASE,
    CAP_NET_RAW,
    CAP_FSETID,
    CAP_SYS_PACCT,
    CAP_SYSLOG,
    CAP_FOWNER,
    CAP_SETGID,
    CAP_WAKE_ALARM,
    CAP_SYS_NICE,
    CAP_NET_BIND_SERVICE,
    CAP_SYS_TIME,
    CAP_SETPCAP,
    CAP_DAC_OVERRIDE,
    CAP_SYS_PTRACE,
    CAP_MAC_OVERRIDE,
    CAP_SYS_CHROOT,
    CAP_SYS_RESOURCE,
    CAP_AUDIT_WRITE,
    CAP_DAC_READ_SEARCH,
    CAP_KILL,
    CAP_AUDIT_CONTROL,
    CAP_IPC_OWNER,
    CAP_BLOCK_SUSPEND,
}
```

The first two output lines show that we're running it as an unprivileged
user: Its set of effective capabilities is empty. The following lines show
the set of effective capabilities that were assigned to the child process:
The full set of capabilities, even though the program was run from an
unprivileged account. When a user namespace is created, the first process in
the namespace is granted a full set of capabilities in the namespace. This
allows that process to perform any initializations that are necessary in
the namespace before other processes are created in the namespace.

The second point of interest is the user and group IDs of the child
process. As noted above, a process's user and group IDs inside and outside
a user namespace can be different. However, there needs to be a mapping
from the user IDs inside a user namespace to a corresponding set of user
IDs outside the namespace; the same is true of group IDs. This allows the
system to perform the appropriate permission checks when a process in a
user namespace performs operations that affect the wider system (e.g.,
sending a signal to a process outside the namespace or accessing a file).

System calls that return process user and group IDs &mdash; for example,
`getuid()` and `getgid()` &mdash; always return credentials as they appear
inside the user namespace in which the calling process resides. If a user
ID has no mapping inside the namespace, then system calls that return user
IDs return the value defined in the file `/proc/sys/kernel/overflowuid`,
which on a standard system defaults to the value 65534. Initially, a user
namespace has no user ID mapping, so all user IDs inside the namespace map
to this value. Likewise, a new user namespace has no mappings for group
IDs, and all unmapped group IDs map to `/proc/sys/kernel/overflowgid`
(which has the same default as `overflowuid`).

There is one other important point worth noting that can't be gleaned from
the output above. Although the new process has a full set of capabilities in
the new user namespace, it has no capabilities in the parent namespace. This
is true regardless of the credentials and capabilities of the process that
calls `clone()`. In particular, even if root employs `clone(CLONE_NEWUSER)`,
the resulting child process will have no capabilities in the parent namespace.

One final point to be made about the creation of user namespaces is that
namespaces can be nested; that is, each user namespace (other than the
initial user namespace) has a parent user namespace, and can have zero or
more child user namespaces. The parent of a user namespace is the user
namespace of the process that creates the user namespace via a call to
`clone()` or `unshare()` with the `CLONE_NEWUSER` flag. The significance of
the parent-child relationship between user namespaces will become clearer
in the remainder of this chapter.

## Mapping user and group IDs

Normally, one of the first steps after creating a new user namespace is
to define the mappings used for the user and group IDs of the processes
that will be created in that namespace. This is done by writing mapping
information to the `/proc/PID/uid_map` and `/proc/PID/gid_map` files
corresponding to one of the processes in the user namespace. (Initially,
these two files are empty.) This information consists of one or more lines,
each of which contains three values separated by white space:

```text
ID-inside-ns   ID-outside-ns   length
```

Together, the `ID-inside-ns` and `length` values define a range of IDs
inside the namespace that are to be mapped to an ID range of the same
length outside the namespace. The `ID-outside-ns` value specifies the
starting point of the outside range. How `ID-outside-ns` is interpreted
depends on the whether the process opening the file `/proc/PID/uid_map`
(or `/proc/PID/gid_map`) is in the same user namespace as the process PID:

* If the two processes are in the same namespace, then `ID-outside-ns` is
  interpreted as a user ID (group ID) in the parent user namespace of the
  process PID. The common case here is that a process is writing to its own
  mapping file (`/proc/self/uid_map` or `/proc/self/gid_map`).
* If the two processes are in different namespaces, then `ID-outside-ns` is
  interpreted as a user ID (resp. group ID) in the user namespace of the
  process opening `/proc/PID/uid_map` (resp. `/proc/PID/gid_map`). The writing
  process is then defining the mapping relative to its own user namespace.

Suppose that we once more invoke our `demo-userns` program, but this time
with a the `--loop` command-line flag. This causes the program to loop,
continuously displaying credentials and capabilities every few seconds:

```text
$ cargo run --bin demo-userns -- --loop
Parent: eUID: 1000, eGID: 1000
Parent: Capabilities (effective): {}
Child: eUID: 65534, eGID: 65534
Child: Capabilities (effective): {
    ...
}
Child: eUID: 65534, eGID: 65534
Child: Capabilities (effective): {
    ...
}
```

Now we switch to another terminal window, to a shell process running in
another namespace &mdash; the parent user namespace of the process running
demo_userns &mdash; and create a user ID mapping for the child process in
the new user namespace created by `demo-userns`:

```text
$ ps -C demo-userns -o 'pid uid comm     # Determine PID of clone child'
  PID   UID COMMAND
 8494  1000 demo-usern                   # This is the parents
 8496  1000 demo-usern                   # Child in a new user namespaces
$ echo '0 1000 1' > /proc/8496/uid_map
```

If we return to the window running demo_userns, we now see:

```text
Child: eUID: 0, eGID: 65534
```

In other words, the user ID 1000 in the parent user namespace (which
was formerly mapped to 65534) has been mapped to user ID 0 in the user
namespace created by `demo-userns` From this point, all operations within
the new user namespace that deal with this user ID will see the number 0,
while corresponding operations in the parent user namespace will see the
same process as having user ID 1000.

We can likewise create a mapping for group IDs in the new user
namespace. Switching to another terminal window, we create a mapping for
the single group ID 1000 in the parent user namespace to the group ID 0 in
the new user namespace:

```text
$ echo '0 1000 1' > /proc/8496/gid_map
-bash: echo: write error: Operation not permitted
```

Oops, something went wrong. As the [`user_namespaces(7)`][user-namespaces]
man page explains, we must first disable the ability to change groups in the
target process &mdash; otherwise changing the `gid_map` will not be allowed:

```text
$ echo deny > /proc/8496/setgroups
$ echo '0 1000 1' > /proc/8496/gid_map
```

Switching back to the window running `demo-userns`, we see that change
reflected in the display of the effective group ID:

```text
Child: eUID: 0, eGID: 0
```


## Rules for writing to mapping files

There are a number of rules governing writing to `uid_map` files; analogous
rules apply for writing to `gid_map` files. The most important of these
rules are as follows.

Defining a mapping is a one-time operation per namespace: We can
perform only a single write (that may contain multiple newline-delimited
records) to a `uid_map` file of exactly one of the processes in the user
namespace. If we try to write again to the `uid_map` file. we fail:

```text
$ echo '0 1000 1' > /proc/8496/uid_map
-bash: echo: write error: Operation not permitted
```

Furthermore, the number of lines that may be written to the file is currentl
limited to 340 (as of Linux 4.15), and the number of bytes written to it
must be less than the system page size.

The `/proc/PID/uid_map` file is owned by the user ID that created the
namespace, and is writeable only by that user (or a privileged user). In
addition, all of the following requirements must be met:

* The writing process must have the `CAP_SETUID` (`CAP_SETGID` for `gid_map`)
  capability in the user namespace of the process PID.
* Regardless of capabilities, the writing process must be in either the
  user namespace of the process PID or inside the (immediate) parent user
  namespace of the process PID.

One of the following must be true:

* The data written to `uid_map` (`gid_map`) consists of a single line that
  maps (only) the writing process's effective user ID (group ID) in the parent
  user namespace to a user ID (group ID) in the user namespace. This rule
  allows the initial process in a user namespace (i.e., the child created by
  `clone()`) to write a mapping for its own user ID (group ID).
* The process has the `CAP_SETUID` (`CAP_SETGID` for `gid_map`) capability in
  the parent user namespace. Such a process can define mappings to arbitrary
  user IDs (group IDs) in the parent user namespace. As we noted earlier,
  the initial process in a new user namespace has no capabilities in the
  parent namespace. Thus, only a process in the parent namespace can write
  a mapping that maps arbitrary IDs in the parent user namespace.

If, as we saw in the previous example, the writing process does not have
the `CAP_SETGID` capability in the _parent_ process, then use of the
[`setgroups(2)`][setgroups] system call must first be denied (by writing
`deny` to `/proc/PID/setgroups`) before writing to `gid_map`.


## Capabilities, `execve()`, and user ID 0

In an earlier chapter, we developed the [`ns-child-exec.rs`][ns-child-exec]
program. This program uses `clone()` to create a child process in new
namespaces specified by command-line options, and then executes a shell
command in the child process.

Suppose that we use this program to execute a shell in a new user namespace,
and then within that shell we try to define the user ID mapping for the
new user namespace. In doing so, we run into a problem:

    $ ./ns_child_exec -U  bash
    $ echo '0 1000 1' > /proc/$$/uid_map       # $$ is the PID of the shell
    bash: echo: write error: Operation not permitted

This error occurs because the shell has no capabilities inside the new user
namespace, as can be seen from the following commands:

    $ id -u         # Verify that user ID and group ID are not mapped
    65534
    $ id -g
    65534
    $ cat /proc/$$/status | egrep 'Cap(Inh|Prm|Eff)'
    CapInh: 0000000000000000
    CapPrm: 0000000000000000
    CapEff: 0000000000000000

The problem occurred at the `execve()` call that executed the Bash shell:
when a process with non-zero user IDs performs an `execve()`, the process's
capability sets are cleared. (The [capabilities(7)][capabilities] manual
page details the treatment of capabilities during an `execve()`.)

To avoid this problem, it is necessary to create a user ID mapping inside
the user namespace before performing the `execve()`. This is not possible
with the `ns-child-exec` program; we need a slightly enhanced version of
the program that does allow this.

The [`userns-child-exec.rs`][userns-child-exec] program performs the
same task as the `ns-child-exec` program, and has the same command-line
interface, except that it allows two additional command-line options, `-M` and
`-G`. These options accept string arguments that are used to define user and
group ID maps for the new user namespace. For example, the following command
maps both user ID 1000 and group ID 1000 to 0 in the new user namespace:

    $ ./userns_child_exec -U -M '0 1000 1' -G '0 1000 1' bash

This time, updating the mapping files succeeds, and we see that the shell
has the expected user ID, group ID, and capabilities:

    $ id -u
    0
    $ id -g
    0
    $ cat /proc/$$/status | egrep 'Cap(Inh|Prm|Eff)'
    CapInh: 0000000000000000
    CapPrm: 0000001fffffffff
    CapEff: 0000001fffffffff

There are some subtleties to the implementation of the `userns-child-exec`
program. First, either the parent process (i.e., the caller of `clone()`) or
the new child process could update the user ID and group ID maps of the new
user namespace. However, following the rules above, the only kind of mapping
that the child process could define would be one that maps just its own
effective user ID. If we want to define arbitrary user and group ID mappings
in the child, then that must be done by the parent process. Furthermore,
the parent process must have suitable capabilities, namely `CAP_SETUID`,
`CAP_SETGID`, and (to ensure that the parent has the permissions needed to
open the mapping files) `CAP_DAC_OVERRIDE`.

Furthermore, the parent must ensure that it updates the mapping files before
the child calls `execve()` (otherwise we have exactly the problem described
above, where the child will lose capabilities during the `execve())`. To do
this, the two processes employ a pipe to ensure the required synchronization;
comments in the program source code give full details.


## Viewing user and group ID mappings

The examples so far showed the use of `/proc/PID/uid_map` and
`/proc/PID/gid_map` files for defining a mapping. These files can also be
used to view the mappings governing a process. As when writing to these
files, the second (`ID-outside-ns`) value is interpreted according to which
process is opening the file. If the process opening the file is in the same
user namespace as the process PID, then `ID-outside-ns` is defined with
respect to the parent user namespace. If the process opening the file is
in a different user namespace, then `ID-outside-ns` is defined with respect
to the user namespace of the process opening the file.

We can illustrate this by creating a couple of user namespaces running shells,
and examining the `uid_map` files of the processes in the namespaces. We
begin by creating a new user namespace with a process running a shell:

    $ id -u            # Display effective user ID
    1000
    $ ./userns_child_exec -U -M '0 1000 1' -G '0 1000 1' bash
    $ echo $$          # Show shell's PID for later reference
    2465
    $ cat /proc/2465/uid_map
             0       1000          1
    $ id -u            # Mapping gives this process an effective user ID of 0
    0

Now suppose we switch to another terminal window and create a sibling user
namespace that employs different user and group ID mappings:

    $ ./userns_child_exec -U -M '200 1000 1' -G '200 1000 1' bash
    $ cat /proc/self/uid_map
           200       1000          1
    $ id -u            # Mapping gives this process an effective user ID of 200
    200
    $ echo $$          # Show shell's PID for later reference
    2535

Continuing in the second terminal window, which is running in the second
user namespace, we view the user ID mapping of the process in the other
user namespace:

    $ cat /proc/2465/uid_map
             0        200          1

The output of this command shows that user ID 0 in the other user namespace
maps to user ID 200 in this namespace. Note that the same command produced
different output when executed in the other user namespace, because the
kernel generates the `ID-outside-ns` value according to the user namespace
of the process that is reading from the file.

If we switch back to the first terminal window, and display the user ID
mapping file for the process in the second user namespace, we see the
converse mapping:

    $ cat /proc/2535/uid_map
           200          0          1

Again, the output here is different from the same command when executed in
the second user namespace, because the `ID-outside-ns` value is generated
according to the user namespace of the process that is reading from the
file. Of course, in the initial namespace, user ID 0 in the first namespace
and user ID 200 in the second namespace both map to user ID 1000. We can
verify this by executing the following commands in a third shell window
inside the initial user namespace:

    $ cat /proc/2465/uid_map
             0       1000          1
    $ cat /proc/2535/uid_map
           200       1000          1


[capabilities]: http://man7.org/linux/man-pages/man7/capabilities.7.html
[demo-userns]: ../src/bin/demo-userns.rs
[ns-child-exec]: ../src/bin/ns-child-exec.rs
[setgroups]: http://man7.org/linux/man-pages/man2/setgroups.2.html
[user-namespaces]: http://man7.org/linux/man-pages/man7/user_namespaces.7.html
[userns-child-exec]: ../src/bin/userns-child-exec.rs
