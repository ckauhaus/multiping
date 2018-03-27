# multiping

This small utility pings several targets at once and returns the lowest RTT.
It can be used as connectivity check to see if we can reach any target from a
list in a configured amount of time. This way, the connectivity check is
independent from the status of any single target host.

This program can be used as Nagios-compatible plugin.


## Example

Ping all known quad9 addresses as well as Google DNS at once:

```sh
$ multiping dns.quad9.net 8.8.8.8
multiping: OK - best rtt 40 ms (for dns.quad9.net/149.112.112.112) | '8.8.8.8'=0.0475s;0.05;0.5;0 '2620:fe::fe'=0.0421s;0.05;0.5;0 '149.112.112.112'=0.0398s;0.05;0.5;0 '9.9.9.9'=0.0411s;0.05;0.5;0
```

The output before the "|" is for humans. Everything after that is
Nagios-compatible perfdata.

Note that multiping needs capabilities to open raw sockets. Running as root is
the easiest way to accomplish this.


## Features

- Fast and robust.
- Output compatible to Nagios/Icinga/Sensu/...
- Select IPv4 or IPv6 only.
- Adjustable warning/critical timeouts.


## Author and License

The primary author is [Christian Kauhaus](kc@flyingcircus.io). `multiping` is
licensed under the term of the
[BSD 3-clause "revised" license](https://opensource.org/licenses/BSD-3-Clause).
