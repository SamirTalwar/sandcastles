use std::io;
use std::net;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Port(pub u16);

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Port {
    pub fn is_in_use(&self) -> bool {
        let socket_address = net::SocketAddrV4::new(net::Ipv4Addr::UNSPECIFIED, self.0);
        let result = net::TcpStream::connect(socket_address);
        result.is_ok()
    }

    pub fn is_available(&self) -> bool {
        !self.is_in_use()
    }

    pub fn next_available() -> io::Result<Self> {
        let socket_address = net::SocketAddrV4::new(net::Ipv4Addr::UNSPECIFIED, 0);
        let port_number = {
            let bound = net::TcpListener::bind(socket_address)?;
            bound.local_addr()?.port()
        };
        thread::yield_now();
        let port = Self(port_number);
        while port.is_in_use() {
            thread::sleep(Duration::from_millis(100));
        }
        Ok(port)
    }
}
