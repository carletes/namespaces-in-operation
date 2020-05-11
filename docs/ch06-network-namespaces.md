# Network namespaces

Network namespaces partition the use of the network &mdash; devices,
addresses, ports, routes, firewall rules, etc. &mdash; &mdash; into separate
boxes, essentially virtualizing the network within a single running kernel
instance. Network namespaces entered the kernel in 2.6.24.

## Basic network namespace management

As with the others, network namespaces are created by passing a flag to
the `clone()` system call: `CLONE_NEWNET`. From the command line, though,
it is convenient to use the [`ip`][iproute2] networking configuration tool
to set up and work with network namespaces. For example:

```text
# ip netns add netns1
```

This command creates a new network namespace called `netns1`. Note that
we ran it as `root`, since creation of network namespaces requires the
`CAP_SYS_ADMIN` capability.

When the `ip` tool creates a network namespace, it will create a bind mount
for it under `/var/run/netns`; that allows the namespace to persist even
when no processes are running within it, and facilitates the manipulation
of the namespace itself:

```text
$ ls -l /var/run/netns
total 0
-r--r--r-- 1 root root 0 May 11 12:07 netns1
```

The `ip netns exec` command can be used to run network management commands
within the namespace. This command, for instance, lists the interfaces
visible inside the namespace:

```text
# ip netns exec netns1 ip link list
1: lo: <LOOPBACK> mtu 65536 qdisc noop state DOWN mode DEFAULT group default qlen 1000
    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00
```

A bind-mount reference to a network namespace can be removed with:

```text
# ip netns delete netns1
```

This command removes the bind mount referring to the given network
namespace. The namespace itself, however, will persist for as long as any
processes are running within it (if there are no more processes running
in it when its bind-mounted reference is deleted, the network namespace is
deleted as well).


### Low-level details

Running the previous commands under the [`strace(1)`][strace] tool shows
which system calls are done by the `ip` command-line tool in order to create
a new network namespace:

```text
# strace -f ip netns add netns1
[..]
openat(AT_FDCWD, "/var/run/netns/netns1", O_RDONLY|O_CREAT|O_EXCL, 000) = 5
close(5)                                = 0
unshare(CLONE_NEWNET)                   = 0
mount("/proc/self/ns/net", "/var/run/netns/netns1", 0x55a4c56f49a5, MS_BIND, NULL) = 0
exit_group(0)                           = ?
+++ exited with 0 +++
```

Here the mount point `/var/run/netns/netns1` is created (as an empty
file), and then the `ip` command creates a new network namespace with the
`unshare()` system call. The file `/proc/self/ns/net` is then bind-mounted to
`/var/run/netns/netns1`, so that the network namespace survives the life-time
of the `ip` command.

Now that we may refer to our new network namespace by its bind-mounted path
`/var/run/netns/netns1`, we may execute processes in it (by passing an open
file descriptor to the [`setns(2)`][setns] system call). The following is
a trace of `/bin/true` running in the `netns1` network namespace:

```text
# strace -f ip netns exec netns1 /bin/true
[..]
openat(AT_FDCWD, "/var/run/netns/netns1", O_RDONLY|O_CLOEXEC) = 5
setns(5, CLONE_NEWNET)                  = 0
close(5)                                = 0
[..]
execve("/bin/true", ["/bin/true"], 0x7ffc1d304d98 /* 35 vars */) = 0
[..]
exit_group(0)                           = ?
+++ exited with 0 +++

```

Deleting the bind-mount-ed reference to this network namespace is done
by unmounting the bind-mount reference to it at `/var/run/netns/netns1`
(and removing the mount point file):

```text
# strace -f ip netns delete netns1
[..]
umount2("/var/run/netns/netns1", MNT_DETACH) = 0
unlink("/var/run/netns/netns1")         = 0
exit_group(0)                           = ?
+++ exited with 0 +++
```

As stated above, note that the network namespace will exist as long as
there are processes running within it.


## Network namespace configuration

New network namespaces will have a loopback device but no other network
devices. Aside from the loopback device, each network device (physical or
virtual interfaces, bridges, etc.) can only be present in a single network
namespace.

Virtual network devices (e.g. virtual ethernet or veth) can be created and
assigned to a namespace. These virtual devices allow processes inside the
namespace to communicate over the network; it is the configuration, routing,
and so on that determine who they can communicate with.

When first created, the `lo` loopback device in the new namespace is down,
so even a loopback ping will fail:

```text
# ip netns exec netns1 ping 127.0.0.1
connect: Network is unreachable
```

Bringing that interface up will allow pinging the loopback address:

```text
# ip netns exec netns1 ip link set dev lo up
# ip netns exec netns1 ping 127.0.0.1
PING 127.0.0.1 (127.0.0.1) 56(84) bytes of data.
64 bytes from 127.0.0.1: icmp_seq=1 ttl=64 time=0.051 ms
...
```

But that still doesn't allow communication between `netns1` and the root
namespace. To do that, virtual ethernet devices need to be created and
configured:

```text
# ip link add veth0 type veth peer name veth1
# ip link set veth1 netns netns1
```

The first command sets up a pair of virtual ethernet devices that are
connected. Packets sent to `veth0` will be received by `veth1` and vice
versa. The second command assigns `veth1` to the `netns1` namespace.

Then, these two commands set IP addresses for the two devices:

```text
# ip netns exec netns1 ifconfig veth1 10.1.1.1/24 up
# ifconfig veth0 10.1.1.2/24 up
```

Communication in both directions is now possible, as the following `ping`
commands show:

```text
# ping 10.1.1.1
PING 10.1.1.1 (10.1.1.1) 56(84) bytes of data.
64 bytes from 10.1.1.1: icmp_seq=1 ttl=64 time=0.087 ms
...

# ip netns exec netns1 ping 10.1.1.2
PING 10.1.1.2 (10.1.1.2) 56(84) bytes of data.
64 bytes from 10.1.1.2: icmp_seq=1 ttl=64 time=0.054 ms
...
```

As mentioned, though, namespaces do not share routing tables or firewall
rules, as running `route` and `iptables -L` in `netns1` will attest.

```text
# ip netns exec netns1 route
Kernel IP routing table
Destination     Gateway         Genmask         Flags Metric Ref    Use Iface
10.1.1.0        0.0.0.0         255.255.255.0   U     0      0        0 veth1
```

Programs running in the `netns1` network namespace will only see the routing
entry above, which routes packets to the interface's subnet through the
other end of the veth interface. As for the firewall rules:

```text
# ip netns exec netns1 iptables -L
Chain INPUT (policy ACCEPT)
target     prot opt source               destination

Chain FORWARD (policy ACCEPT)
target     prot opt source               destination

Chain OUTPUT (policy ACCEPT)
target     prot opt source               destination
```

The lack of a default inside the network namespace `netns1` means that
no network connections to any address outside the veth pair subnet are
possible. There are several ways to connect the namespace to the internet
if that is desired. A bridge can be created in the root namespace and the
veth device from `netns1`. Alternatively, IP forwarding coupled with network
address translation (NAT) could be configured in the root namespace. Either of
those (and there are other configuration possibilities) will allow packets
from `netns1` to reach the internet and for replies to be received in
`netns1`.

Non-root processes that are assigned to a namespace (via `clone()`,
`unshare()`, or `setns()`) only have access to the networking devices and
configuration that have been set up in that namespace &mdash; user `root`
can add new devices and configure them, of course. Using the `ip netns`
sub-command, there are two ways to address a network namespace: by its name,
like `netns1`, or by the process ID of a process in that namespace. Since
`init` generally lives in the root namespace, one could use a command like:

```text
# ip link set vethX netns 1
```

That would put a (presumably newly created) veth device into the root
namespace and it would work for a root user from any other namespace. In
situations where it is not desirable to allow root to perform such operations
from within a network namespace, the PID and mount namespace features can
be used to make the other network namespaces unreachable.


## Uses for network namespaces

As we have seen, a namespace's networking can range from none at all (or
just loopback) to full access to the system's networking capabilities. That
leads to a number of different use cases for network namespaces.

By essentially turning off the network inside a namespace, administrators
can ensure that processes running there will be unable to make connections
outside of the namespace. Even if a process is compromised through some
kind of security vulnerability, it will be unable to perform actions like
joining a botnet or sending spam.

Even processes that handle network traffic (a web server worker process or
web browser rendering process for example) can be placed into a restricted
namespace. Once a connection is established by or to the remote endpoint,
the file descriptor for that connection could be handled by a child process
that is placed in a new network namespace created by a `clone()` call. The
child would inherit its parent's file descriptors, thus have access to
the connected descriptor. Another possibility would be for the parent to
send the connected file descriptor to a process in a restricted network
namespace via a Unix socket. In either case, the lack of suitable network
devices in the namespace would make it impossible for the child or worker
process to make additional network connections.

Namespaces could also be used to test complicated or intricate networking
configurations all on a single box. Running sensitive services in more
locked-down, firewall-restricted namespace is another. Obviously, container
implementations also use network namespaces to give each container its own
view of the network, untrammeled by processes outside of the container.


[iproute2]: https://wiki.linuxfoundation.org/networking/iproute2
[setns]: http://man7.org/linux/man-pages/man2/setns.2.html
[strace]: http://man7.org/linux/man-pages/man1/strace.1.html
