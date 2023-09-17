use std::io;
use std::net;
use std::thread;

use crate::timing::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Port(pub u16);

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Port {
    const DYNAMIC: Self = Self(0);

    pub fn is_in_use(&self) -> bool {
        let result = net::TcpStream::connect(self.localhost());
        result.is_ok()
    }

    pub fn is_available(&self) -> bool {
        !self.is_in_use()
    }

    pub fn next_available() -> io::Result<Self> {
        let port_number = {
            let bound = net::TcpListener::bind(Self::DYNAMIC.localhost())?;
            bound.local_addr()?.port()
        };
        thread::yield_now();
        let port = Self(port_number);
        while port.is_in_use() {
            Duration::QUANTUM.sleep();
        }
        Ok(port)
    }

    fn localhost(&self) -> net::SocketAddr {
        net::SocketAddr::V6(net::SocketAddrV6::new(
            net::Ipv6Addr::LOCALHOST,
            self.0,
            0,
            0,
        ))
    }
}
