use super::*;
use status::Status::*;

/// Helper to quicky parse an IP address literal
fn addr(a: &str) -> IpAddr {
    a.parse::<IpAddr>().expect("invalid IP address")
}

/// Helper to fake PingTimes results
fn pt<'a>(data: &[(&'a str, Option<f64>)]) -> PingTimes<'a> {
    let mut t = PingTimes::default();
    for &(tgt, time) in data {
        t.targets.addr.push(addr(tgt));
        t.targets.host.push(tgt);
        t.times.push(time);
    }
    t
}

#[test]
fn build_targets_ip4_literal() {
    let t = Targets::build(vec!["8.8.8.8"], is_any);
    assert_eq!(t.unwrap().addr, vec![addr("8.8.8.8")]);
}

#[test]
fn build_targets_ip6_literal() {
    let t = Targets::build(vec!["2620:fe::fe"], is_any);
    assert_eq!(t.unwrap().addr, vec![addr("2620:fe::fe")]);
}

#[test]
#[ignore]
fn build_targets_dualstack() {
    let t = Targets::build(vec!["8.8.8.8", "localhost"], is_any);
    assert_eq!(
        t.unwrap().addr,
        vec![addr("8.8.8.8"), addr("::1"), addr("127.0.0.1")]
    );
}

#[test]
fn build_targets_ipv4() {
    let t = Targets::build(vec!["localhost"], IpAddr::is_ipv4);
    assert_eq!(t.unwrap().addr, vec![addr("127.0.0.1")]);
}

#[test]
#[ignore]
fn build_targets_ipv6() {
    let t = Targets::build(vec!["localhost"], IpAddr::is_ipv6);
    assert_eq!(t.unwrap().addr, vec![addr("::1")]);
}

#[test]
fn build_targets_resolve_error() {
    assert!(Targets::build(vec!["no.such.host.example.com"], is_any).is_err());
}

#[test]
fn eval_no_data() {
    assert_eq!(
        pt(&[]).evaluate(0., 0.),
        ("no targets found".into(), Unknown)
    );
}

#[test]
fn eval_all_timeout() {
    assert_eq!(
        pt(&[("8.8.8.8", None), ("4.4.4.4", None)]).evaluate(0.3, 0.4),
        (
            "no data | '8.8.8.8'=Us;0.3;0.4;0 '4.4.4.4'=Us;0.3;0.4;0".into(),
            Critical
        )
    );
}

#[test]
fn eval_ok() {
    assert_eq!(
        pt(&[("8.8.8.8", Some(0.01)), ("4.4.4.4", None)]).evaluate(0.1, 0.2),
        (
            "best rtt 10 ms (for 8.8.8.8) | '8.8.8.8'=0.01s;0.1;0.2;0 '4.4.4.4'=Us;0.1;0.2;0"
                .into(),
            OK
        )
    );
}

#[test]
fn eval_ok_fmt_hostnames() {
    let mut t = PingTimes::default();
    t.targets.addr.push(addr("8.8.8.8"));
    t.targets.host.push("google.ns");
    t.times.push(Some(0.054));
    assert_eq!(
        t.evaluate(0.1, 0.2),
        (
            "best rtt 54 ms (for google.ns/8.8.8.8) | '8.8.8.8'=0.054s;0.1;0.2;0".into(),
            OK
        )
    );
}

#[test]
fn eval_warning() {
    assert_eq!(pt(&[("8.8.8.8", Some(1.0))]).evaluate(0.1, 1.0).1, Warning);
}

#[test]
fn eval_critical() {
    assert_eq!(pt(&[("8.8.8.8", Some(1.1))]).evaluate(0.1, 1.0).1, Critical);
}

#[test]
fn min_rtt_empty() {
    assert!(pt(&[]).min_rtt().is_none())
}

#[test]
fn min_rtt_regular() {
    assert_eq!(
        pt(&[
            ("1.1.1.1", Some(0.3)),
            ("2.2.2.2", Some(0.2)),
            ("3.3.3.3", Some(0.1)),
        ]).min_rtt(),
        Some((0.1, "3.3.3.3", addr("3.3.3.3")))
    )
}

#[test]
fn min_rtt_filter_out_none() {
    assert_eq!(
        pt(&[("1.1.1.1", None), ("2.2.2.2", Some(0.2))]).min_rtt(),
        Some((0.2, "2.2.2.2", addr("2.2.2.2")))
    )
}

#[test]
fn min_rtt_nan() {
    assert_eq!(
        pt(&[("1.1.1.1", Some(0. / 0.)), ("2.2.2.2", Some(0.2))]).min_rtt(),
        Some((0.2, "2.2.2.2", addr("2.2.2.2")))
    )
}
