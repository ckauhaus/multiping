//! Asynchronous pings.

use futures::future::{join_all, ok};
use futures::prelude::*;
use errors::*;
use std::net::IpAddr;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use tokio_core::reactor;
use tokio_ping::{self, Pinger};

/// Number of ping attempts before giving up
const MAX_PER_TARGET: u64 = 5;

pub type Times = Vec<Option<f64>>;

/// Collects ping times for each target. Ping is stopped either if the best time is below `cutoff`
/// or if `MAX_PER_TARGET` attempts are taken.
fn measure<S>(reactor: &mut reactor::Core, targets: Vec<S>, cutoff: f64) -> Result<Times>
where
    S: Stream<Item = Option<f64>, Error = tokio_ping::Error> + 'static,
{
    let best: Rc<RefCell<Times>> = Rc::new(RefCell::new(vec![None; targets.len()]));
    {
        let f = targets.into_iter().enumerate().map(|(i, target)| {
            let best = best.clone();
            target
                .take(MAX_PER_TARGET)
                .filter_map(|elt| elt) // take out None values
                .take_while(move |elt| {
                    let mut best = best.borrow_mut();
                    match best[i] {
                        Some(b) if b > *elt => best[i] = Some(*elt),
                        None                => best[i] = Some(*elt),
                        _ => (),
                    };
                    ok(*elt >= cutoff)
                })
                .for_each(|_| Ok(()))
        });
        reactor.run(join_all(f))?;
    }
    Ok(Rc::try_unwrap(best).unwrap().into_inner())
}

/// Sets up async core and starts parallel ping.
pub fn ping_all<'a, I>(targets: I, cutoff: f64) -> Result<Times>
where
    I: Iterator<Item = &'a IpAddr>,
{
    let mut reactor = reactor::Core::new().unwrap();
    let hdl = reactor.handle();
    let streams = targets
        .map(|addr| Pinger::new(&hdl).map(|p| p.chain(*addr).stream()))
        .collect::<io::Result<_>>()
        .chain_err(|| "cannot create ICMP socket - missing privileges?".to_string())?;
    measure(&mut reactor, streams, cutoff)
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::stream;
    use std::vec::IntoIter;

    fn fake_stream<'a>(
        ping_times: &'a [&'a [Option<f64>]],
    ) -> Vec<stream::IterOk<IntoIter<Option<f64>>, tokio_ping::Error>> {
        let mut streams = Vec::new();
        for timeseries in ping_times {
            let times: Times = timeseries.iter().cloned().collect();
            streams.push(stream::iter_ok(times.into_iter()))
        }
        streams
    }

    fn r() -> reactor::Core {
        reactor::Core::new().unwrap()
    }

    #[test]
    fn test_singletons() {
        let times = fake_stream(&[&[Some(54.1)], &[Some(0.2)]]);
        assert_eq!(
            measure(&mut r(), times, 1e2).unwrap(),
            vec![Some(54.1), Some(0.2)]
        )
    }

    #[test]
    fn test_collect_minimum() {
        let times = fake_stream(&[
            // should pick middle one
            &[Some(54.1), Some(53.0), Some(53.5)],
            // should pick last one
            &[None, Some(0.3), Some(0.2)],
            // should pick first one
            &[Some(1.0), Some(2.5), Some(3.0)],
            // should pick nothing
            &[None, None, None],
        ]);
        assert_eq!(
            measure(&mut r(), times, 1e-2).unwrap(),
            vec![Some(53.0), Some(0.2), Some(1.0), None]
        )
    }

    #[test]
    fn test_max_attempts() {
        let mut rtt = vec![Some(4.0); MAX_PER_TARGET as usize];
        rtt.push(Some(2.0));
        let times = fake_stream(&[&rtt]);
        assert_eq!(measure(&mut r(), times, 1e2).unwrap(), vec![Some(4.0)]);
    }

    #[test]
    fn test_stop_single_target_below_cutoff() {
        // should pick 3rd one (first below cutoff
        let times = fake_stream(&[&[Some(9.0), Some(8.0), Some(7.0), Some(6.0)]]);
        assert_eq!(measure(&mut r(), times, 8.0).unwrap(), vec![Some(7.0)]);
    }

    #[test]
    fn test_multi_cutoff() {
        // should pick 3rd one (first below cutoff
        let times = fake_stream(&[
            &[Some(7.0), Some(6.0), Some(5.0), Some(4.0)],
            &[Some(8.0), Some(7.0), Some(6.0), Some(5.0)],
            // not cutoff
            &[Some(9.0), Some(8.0), Some(7.0), Some(6.0)],
        ]);
        assert_eq!(
            measure(&mut r(), times, 5.1).unwrap(),
            vec![Some(5.0), Some(5.0), Some(6.0)]
        );
    }

}
