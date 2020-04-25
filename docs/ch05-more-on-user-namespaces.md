# More on user namespaces

In this chapter we look in more detail at the interaction of user namespaces
and capabilities, as well as the combination of user namespaces with other
types of namespaces.


## User namespaces and capabilities

Each process is associated with a particular user namespace. A process created
by a call to `fork()` or a call to `clone()` without the `CLONE_NEWUSER`
flag is placed in the same user namespace as its parent process. A process
can change its user namespace membership using [`setns()`][setns], if it
has the `CAP_SYS_ADMIN` capability in the target namespace; in that case,
it obtains a full set of capabilities upon entering the target namespace.

On the other hand, a `clone(CLONE_NEWUSER)` call creates a new user namespace
and places the new child process in that namespace. This call also establishes
a parental relationship between the two namespaces: each user namespace
(other than the initial namespace) has a parent &mdash; the user namespace
of the process that created it using `clone(CLONE_NEWUSER)`. A parental
relationship between user namespaces is also established when a process calls
`unshare(CLONE_NEWUSER)`. The difference is that `unshare()` places the
caller in the new user namespace, and the parent of that namespace is the
caller's previous user namespace. As we'll see in a moment, the parental
relationship between user namespaces is important because it defines the
capabilities that a process may have in a child namespace.

Each process also has three associated sets of capabilities: permitted,
effective, and inheritable. The [capabilities manual page][capabilities]
describes these three sets in some detail. In this chapter, it is mainly
the effective capability set that is of interest to us. This set determines
a process's ability to perform privileged operations.

User namespaces change the way in which (effective) capabilities are
interpreted. First, having a capability inside a particular user namespace
allows a process to perform operations only on resources governed by
that namespace; we say more on this point below, when we talk about the
interaction of user namespaces with other types of namespaces. In addition,
whether or not a process has capabilities in a particular user namespace
depends on its namespace membership and the parental relationship between
user namespaces. The rules are as follows:

1. A process has a capability inside a user namespace if it is a member of
   the namespace and that capability is present in its effective capability
   set. A process may obtain capabilities in its effective set in a number of
   ways. The most common reasons are that it executed a program that conferred
   capabilities (a set-user-ID program or a program that has associated file
   capabilities) or it is the child of a call to `clone(CLONE_NEWUSER)`,
   which automatically obtains a full set of capabilities.
2. If a process has a capability in a user namespace, then it has that
   capability in all child (and further removed descendant) namespaces as
   well. Put another way: creating a new user namespace does not isolate the
   members of that namespace from the effects of privileged processes in a
   parent namespace.
3. When a user namespace is created, the kernel records the effective user
   ID of the creating process as being the "owner" of the namespace. A process
   whose effective user ID matches that of the owner of a user namespace
   and which is a member of the parent namespace has all capabilities in the
   namespace. By virtue of the previous rule, those capabilities propagate down
   into all descendant namespaces as well. This means that after creation of
   a new user namespace, other processes owned by the same user in the parent
   namespace have all capabilities in the new namespace.

We can demonstrate the third rule with the help of a small program,
[`userns-setns-test.rs`][userns-setns-test]. This program takes one
command-line argument: the pathname of a `/proc/PID/ns/user` file that
identifies a user namespace. It creates a child in a new user namespace
and then both the parent (which remains in the same user namespace as the
shell that was used to invoke the program) and the child attempt to join the
namespace specified on the command line using `setns()`; as noted above,
`setns()` requires that the caller have the `CAP_SYS_ADMIN` capability in
the target namespace.

For our demonstration, we use this program in conjunction with the
[`userns-child-exec.rs`][userns-child-exec] program developed in the previous
chapter. First, we use that program to start a shell (we use `bash`, simply
to create a distinctively named process) running in a new user namespace:

```text
$ id -u
1000
$ readlink /proc/$$/ns/user
user:[4026531837]
$ cargo run --bin userns-child-exec -- --user --uid-map '0 1000 1' --gid-map '0 1000 1' -- /bin/bash
$ echo $$
4340
$ readlink /proc/$$/ns/user
user:[4026532631]
```

Now, we switch to a separate terminal window, to a shell running in the
initial namespace, and run our test program:

```text
$ readlink /proc/$$/ns/user           # Verify that we are in parent namespace
user:[4026531837]
$ cargo run --bin userns-setns-test /proc/4340/ns/user
parent: readlink("/proc/self/ns/user"): user:[4026531837]
parent: setns() succeeded
child: readlink("/proc/self/ns/user"): user:[4026532630]
child: setns() failed: EPERM: Operation not permitted
```

Looking at the output of the `readlink` commands at the start of each
shell session, we can see that the parent process created when the
`userns-setns-test` program was run is in the initial user namespace
(4026531837). (As noted in an earlier chapetr, these numbers are i-node
numbers for symbolic links in the `/proc/PID/ns directory`.) As such, by
rule three above, since the parent process had the same effective user ID
(1000) as the process that created the new user namespace (4026532631), it
had all capabilities in that namespace, including `CAP_SYS_ADMIN`; thus the
`setns()` call in the parent succeeds.

On the other hand, the child process created by `userns-setns-test` is in
a different namespace (4026532630) &mdash; in effect, a sibling namespace
of the namespace where the `bash process is running. As such, the second of
the rules described above does not apply, because that namespace is not an
ancestor of namespace 4026532318. Thus, the child process does not have the
`CAP_SYS_ADMIN` capability in that namespace and the `setns()` call fails.


## Combining user namespaces with other types of namespaces

Creating namespaces other than user namespaces requires the `CAP_SYS_ADMIN`
capability. On the other hand, creating a user namespace requires (since
Linux 3.8) no capabilities, and the first process in the namespace gains a
full set of capabilities (in the new user namespace). This means that that
process can now create any other type of namespace using a second call to
`clone()`.

However, this two-step process is not necessary. It is also possible to
include additional `CLONE_NEW*` flags in the same `clone()` (or `unshare()`)
call that employs `CLONE_NEWUSER` to create the new user namespace. In this
case, the kernel guarantees that the `CLONE_NEWUSER` flag is acted upon
first, creating a new user namespace in which the to-be-created child has
all capabilities. The kernel then acts on all of the remaining `CLONE_NEW*`
flags, creating corresponding new namespaces and making the child a member
of all of those namespaces.

Thus, for example, an unprivileged process can make a call of the following
form to create a child process that is a member of both a new user namespace
and a new UTS namespace:

```text
clone(child_func, stackp, CLONE_NEWUSER | CLONE_NEWUTS, arg);
```

We can use our `userns-child-exec` program to perform a `clone()` call
equivalent to the above and execute a shell in the child process. The
following command specifies the creation of a new UTS namespace and a new
user namespace in which both user and group ID 1000 are mapped to 0:

```text
$ uname -n           # Display hostname for later reference
rilke
$ cargo run --bin userns-child-exec -- --user --uts --uid-map '0 1000 1' --gid-map '0 1000 1' -- bash
```

As expected, the shell process has a full set of permitted and effective
capabilities:

```text
$ id -u
0
$ id -g
0
$ cat /proc/$$/status | egrep 'Cap(Inh|Prm|Eff)'
CapInh: 0000000000000000
CapPrm: 0000003fffffffff
CapEff: 0000003fffffffff
```

In the above output, the hexadecimal value 3fffffffff represents a capability
set in which all of the currently available Linux capabilities are enabled.

We can now go on to modify the hostname &mdash; one of the global resources
isolated by UTS namespaces &mdash; using the standard hostname command; that
operation requires the `CAP_SYS_ADMIN` capability. First, we set the hostname
to a new value, and then we review that value with the `uname` command:

```text
$ hostname pepe     # Update hostname in this UTS namespace
$ uname -n          # Verify the change
pepe
```

Switching to another terminal window &mdash; one that is running in the
initial UTS namespace &mdash; we then check the hostname in that UTS
namespace:

```text
$ uname -n          # Hostname in original UTS namespace is unchanged
rilke
```

From the above output, we can see that the change of hostname in the child
UTS namespace is not visible in the parent UTS namespace.


## Capabilities revisited

Although the kernel grants all capabilities to the initial process in a user
namespace, this does not mean that process then has superuser privileges
within the wider system. (It may, however, mean that unprivileged users now
have access to exploits in kernel code that was formerly accessible only to
root, as [this mail][tmpfs-use-after-free] on a vulnerability in tmpfs mounts
notes.) When a new IPC, mount, network, PID, or UTS namespace is created
via `clone()` or `unshare()`, the kernel records the user namespace of the
creating process against the new namespace. Whenever a process operates on
global resources governed by a namespace, permission checks are performed
according to the process's capabilities in the user namespace that the
kernel associated with the that namespace.

For example, suppose that we create a new user namespace using
`clone(CLONE_NEWUSER)`. The resulting child process will have a full set of
capabilities in the new user namespace, which means that it will, for example,
be able to create other types of namespaces and be able to change its user
and group IDs to other IDs that are mapped in the namespace. (In the previous
chapter, we saw that only a privileged process in the parent user namespace
can create mappings to IDs other than the effective user and group ID of the
process that created the namespace, so there is no security loophole here.)

On the other hand, the child process would not be able to mount a
filesystem. The child process is still in the initial mount namespace, and
in order to mount a filesystem in that namespace, it would need to have
capabilities in the user namespace associated with that mount namespace
(i.e., it would need capabilities in the initial user namespace), which it
does not have. Analogous statements apply for the global resources isolated
by IPC, network, PID, and UTS namespaces.

Furthermore, the child process would not be able to perform privileged
operations that require capabilities that are not (currently) governed by
namespaces. Thus, for example, the child could not do things such as raising
its hard resource limits, setting process priorities, or loading kernel
modules. All of those operations require capabilities that sit outside
the user namespace hierarchy; in effect, those operations require that the
caller have capabilities in the initial user namespace.

By isolating the effect of capabilities to namespaces, user namespaces
thus deliver on the promise of safely allowing unprivileged users access to
functionality that was formerly limited to the root user. This in turn creates
interesting possibilities for new kinds of user-space applications. For
example, it now becomes possible for unprivileged users to run Linux
containers without root privileges, to construct Chrome-style sandboxes
without the use of set-user-ID-root helpers, to implement fakeroot-type
applications without employing dynamic-linking tricks, and to implement
chroot()-based applications for process isolation. Barring kernel bugs,
applications that employ user namespaces to access privileged kernel
functionality are more secure than traditional applications based
on set-user-ID-root: with a user-namespace-based approach, even if an
applications is compromised, it does not have any privileges that can be
used to do damage in the wider system.


[capabilities]: http://man7.org/linux/man-pages/man7/capabilities.7.html
[setns]: http://man7.org/linux/man-pages/man2/setns.2.html
[userns-child-exec]: ../src/bin/userns-child-exec.rs
[userns-setns-test]: ../src/bin/user-setns-test.rs
[tmpfs-use-after-free]: https://lwn.net/Articles/540083/
