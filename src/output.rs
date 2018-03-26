//! Output formatting utilities.

use std::borrow::Cow;

/// Formats option float value as String
///
/// The Nagios Plugin Developer Guidelines require that nonexistent values are displayed as single
/// letter "U".
pub fn u(val: &Option<f64>) -> Cow<'static, str> {
    if let Some(num) = *val {
        Cow::from(num.to_string())
    } else {
        Cow::from("U")
    }
}

/// Formats ping target depending on the presence of both host/addr or addr only.
pub fn best(host: &str, addr: String) -> String {
    if host == addr {
        addr
    } else {
        format!("{}/{}", host, addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u_value() {
        assert_eq!(u(&Some(1.23)), "1.23");
        assert_eq!(u(&None), "U");
    }

    #[test]
    fn best_addr_host() {
        assert_eq!(best("1.2.3.4", "1.2.3.4".into()), "1.2.3.4");
        assert_eq!(
            best("dns.quad9.net", "9.9.9.9".into()),
            "dns.quad9.net/9.9.9.9"
        );
    }
}
