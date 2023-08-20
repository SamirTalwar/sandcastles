use std::net;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Port(pub u16);

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Port {
    pub fn is_available(&self) -> bool {
        let socket_address = net::SocketAddrV4::new(net::Ipv4Addr::UNSPECIFIED, self.0);
        let result = net::TcpListener::bind(socket_address);
        result.is_ok()
    }

    pub fn is_in_use(&self) -> bool {
        !self.is_available()
    }
}
