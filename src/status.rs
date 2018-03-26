use std::fmt;
use self::Status::*;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Status {
    OK,
    Warning,
    Critical,
    Unknown,
}

impl Status {
    pub fn check(val: f64, warn: f64, crit: f64) -> Self {
        if val < 0. {
            panic!("Status::check not implemented for negative values: {}", val);
        }
        match val {
            v if v > crit => Critical,
            v if v > warn => Warning,
            v if v <= warn => OK,
            _ => Unknown,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                OK => "OK",
                Warning => "WARNING",
                Critical => "CRITICAL",
                Unknown => "UNKNOWN",
            }
        )
    }
}

#[test]
fn test_check() {
    assert_eq!(Status::check(0.1, 0.1, 0.2), OK);
    assert_eq!(Status::check(0.2, 0.1, 0.2), Warning);
    assert_eq!(Status::check(0.21, 0.1, 0.2), Critical);
    assert_eq!(Status::check(0. / 0., 0.1, 0.2), Unknown);
}

#[test]
#[should_panic]
fn test_check_neg() {
    Status::check(-1., 1., 2.);
}
