
pub mod udp {
    /// Bind UDP Socket to the specified address
    ///
    /// The function does the same than UdpSocket::bind() and
    /// additionally set reuse address and port socket options.
    pub fn bind(addr: std::net::SocketAddr) -> Result<std::net::UdpSocket, std::io::Error> {
        let domain = if addr.is_ipv6() {
            socket2::Domain::IPV6
        } else {
            socket2::Domain::IPV4
        };
        let addr = socket2::SockAddr::from(addr);
        let s = socket2::Socket::new(domain, socket2::Type::DGRAM, None)?;
        s.set_reuse_address(true)?;
        s.set_reuse_port(true)?;
        s.bind(&addr)?;
        Ok(std::net::UdpSocket::from(s))
    }
}
