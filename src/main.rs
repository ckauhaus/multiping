#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate tokio_core;
extern crate tokio_ping;

mod engine;
mod output;
mod status;
#[cfg(test)]
mod tests;

mod errors {
    error_chain! {
        links {
            Ping(::tokio_ping::Error, ::tokio_ping::ErrorKind);
        }

        foreign_links {
            Io(::std::io::Error);
            Clap(::clap::Error);
        }
    }
}

use engine::{ping_all, Times};
use error_chain::ChainedError;
use errors::*;
use status::Status;
use std::fmt::Write;
use std::net::{IpAddr, ToSocketAddrs};
use std::process;

/// Transparent AF filter
fn is_any(_: &IpAddr) -> bool {
    true
}

/// List of ping target addresses
///
/// We keep a reference to the original command line argument for output. If a numeric target was
/// given on the command line, 'host' equials the ASCII rendition of 'addr' and will be collapsed
/// on output.
#[derive(Clone, Debug, Default)]
struct Targets<'a> {
    host: Vec<&'a str>,
    addr: Vec<IpAddr>,
    warn: Vec<String>,
}

impl<'a> Targets<'a> {
    /// Resolves single host name and adds results to addr, host, warn
    fn add_host(&mut self, host: &'a str, filt: fn(&IpAddr) -> bool) {
        match (host, 0).to_socket_addrs() {
            Ok(addrs) => {
                for addr in addrs.map(|sa| sa.ip()).filter(filt) {
                    self.addr.push(addr);
                    self.host.push(host);
                }
            }
            Err(e) => self.warn.push(format!("{}: {}", host, e)),
        }
    }

    /// Resolves and filters by AF
    fn build<I>(hosts: I, filt: fn(&IpAddr) -> bool) -> Result<Self>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut t = Self::default();
        for host in hosts {
            t.add_host(host, filt);
        }
        Ok(t)
    }

    /// Actually invokes the ping machinery and feeds results to the next stage.
    fn ping(self, cutoff: f64) -> Result<PingTimes<'a>> {
        let times = ping_all(self.addr.iter(), cutoff)?;
        Ok(PingTimes {
            targets: self,
            times,
        })
    }
}

/// Top-level data structure for results
#[derive(Clone, Debug, Default)]
struct PingTimes<'a> {
    targets: Targets<'a>,
    times: Times,
}

impl<'a> PingTimes<'a> {
    /// Finds the target with the minimal ping rtt
    fn min_rtt(&self) -> Option<(f64, &'a str, IpAddr)> {
        self.times
            .iter()
            .enumerate()
            .filter_map(|(i, elt)| elt.map(|t| (t, self.targets.host[i], self.targets.addr[i])))
            .filter(|&(time, _, _)| !time.is_nan())
            .min_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Formats performance data in a Nagios-compatible way (without leading "|")
    fn perfdata(&self, warn: f64, crit: f64) -> String {
        let mut res = String::with_capacity(self.times.len() * 20);
        for (i, val) in self.times.iter().enumerate() {
            write!(
                &mut res,
                " '{}'={:.6}s;{};{};0",
                self.targets.addr[i],
                output::u(val),
                warn,
                crit
            )
            .ok();
        }
        res
    }

    /// Generates Nagios-compatible output and status code
    fn evaluate(self, warn: f64, crit: f64) -> (String, Status) {
        let (mut output, status) = if self.times.is_empty() {
            ("no targets found".into(), Status::Unknown)
        } else if let Some((best_time, best_host, best_addr)) = self.min_rtt() {
            (
                format!(
                    "best rtt {:.0} ms (for {}) |{}",
                    best_time * 1e3,
                    output::best(best_host, best_addr.to_string()),
                    self.perfdata(warn, crit)
                ),
                Status::check(best_time, warn, crit),
            )
        } else {
            (
                format!("no data |{}", self.perfdata(warn, crit)),
                Status::Critical,
            )
        };
        for warning in self.targets.warn {
            write!(output, "\nwarning: {}", warning).is_ok();
        }
        (output, status)
    }
}

fn run() -> Result<i32> {
    use clap::Arg;
    let args = app_from_crate!()
        .about("Pings several hosts at once to test outside connectivity")
        .long_about(crate_description!())
        .arg(
            Arg::with_name("warn_ms")
                .short("w")
                .long("warning")
                .default_value("50")
                .help("WARN if no target's rtt is below"),
        )
        .arg(
            Arg::with_name("crit_ms")
                .short("c")
                .long("critical")
                .default_value("500")
                .help("CRIT if no target's rtt is below"),
        )
        .arg(
            Arg::with_name("ipv4")
                .short("4")
                .long("ipv4")
                .conflicts_with("ipv6")
                .help("Ping only IPv4 addresses"),
        )
        .arg(
            Arg::with_name("ipv6")
                .short("6")
                .long("ipv6")
                .help("Ping only IPv6 addresses"),
        )
        .arg(
            Arg::with_name("TARGET")
                .required(true)
                .multiple(true)
                .help("Ping targets (hostname or IP address)"),
        )
        .get_matches();

    let warn = value_t!(args, "warn_ms", f64)? * 1e-3;
    let crit = value_t!(args, "crit_ms", f64)? * 1e-3;
    let af_filter = match (args.is_present("ipv4"), args.is_present("ipv6")) {
        (true, false) => IpAddr::is_ipv4,
        (false, true) => IpAddr::is_ipv6,
        (_, _) => is_any,
    };
    let (output, status) = Targets::build(
        args.values_of("TARGET")
            .expect("required arg HOSTS missing"),
        af_filter,
    )?
    .ping(warn)?
    .evaluate(warn, crit);
    println!("{}: {} - {}", crate_name!(), status, output);
    Ok(status as i32)
}

fn main() {
    match run() {
        Ok(exit) => process::exit(exit),
        Err(err) => {
            eprint!("{}: {}", crate_name!(), err.display_chain());
            process::exit(3);
        }
    }
}
