//! Is a forwarded host port already taken? The same test Vagrant makes
//! before `up` (`handle_forwarded_port_collisions.rb`, 2.4.9): a TCP connect
//! to the host IP, where "all interfaces" is tested through loopback here.
//! Only TCP is checked, as Vagrant skips UDP ports.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use serde::Deserialize;
use tokio::net::TcpStream;
use tokio::task::JoinSet;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPort {
    #[serde(default)]
    pub host_ip: Option<String>,
    pub port: u16,
}

/// Vagrant waits 0.1 s; a listening local port answers far sooner. Without a
/// short limit, Windows takes about 2 s to refuse a closed localhost port.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);

/// One answer per port, in order: whether something accepts TCP connections
/// on it. Ports are tested in parallel.
pub async fn ports_in_use(ports: &[HostPort]) -> Vec<bool> {
    let mut tasks = JoinSet::new();
    for (i, p) in ports.iter().enumerate() {
        let addr = connect_addr(p);
        tasks.spawn(async move {
            let open = match addr {
                Some(addr) => matches!(
                    tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await,
                    Ok(Ok(_))
                ),
                None => false,
            };
            (i, open)
        });
    }
    let mut out = vec![false; ports.len()];
    while let Some(result) = tasks.join_next().await {
        if let Ok((i, open)) = result {
            out[i] = open;
        }
    }
    out
}

/// `None` for an invalid host IP (the editor reports that separately).
fn connect_addr(p: &HostPort) -> Option<SocketAddr> {
    let ip = match p
        .host_ip
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => IpAddr::from([127, 0, 0, 1]),
        Some(s) => s.parse().ok()?,
    };
    let ip = match ip {
        IpAddr::V4(v4) if v4.is_unspecified() => IpAddr::from([127, 0, 0, 1]),
        IpAddr::V6(v6) if v6.is_unspecified() => IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
        ip => ip,
    };
    Some(SocketAddr::new(ip, p.port))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port(host_ip: Option<&str>, port: u16) -> HostPort {
        HostPort {
            host_ip: host_ip.map(Into::into),
            port,
        }
    }

    #[test]
    fn all_interfaces_are_tested_through_loopback() {
        assert_eq!(
            connect_addr(&port(None, 8080)).unwrap().to_string(),
            "127.0.0.1:8080"
        );
        assert_eq!(
            connect_addr(&port(Some(""), 80)).unwrap().to_string(),
            "127.0.0.1:80"
        );
        assert_eq!(
            connect_addr(&port(Some("0.0.0.0"), 80))
                .unwrap()
                .to_string(),
            "127.0.0.1:80"
        );
        assert_eq!(
            connect_addr(&port(Some("::"), 80)).unwrap().to_string(),
            "[::1]:80"
        );
        assert_eq!(
            connect_addr(&port(Some("192.168.1.5"), 80))
                .unwrap()
                .to_string(),
            "192.168.1.5:80"
        );
        assert_eq!(connect_addr(&port(Some("nope"), 80)), None);
    }

    #[tokio::test]
    async fn a_listening_port_is_in_use() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let taken = listener.local_addr().unwrap().port();
        let free = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let result = ports_in_use(&[
            port(None, taken),
            port(Some("127.0.0.1"), free),
            port(Some("x"), taken),
        ])
        .await;
        assert_eq!(result, [true, false, false]);
    }
}
