use std::net::TcpListener;

/// Get a random available port number.
pub fn random_port() -> u16 {
    TcpListener::bind("0.0.0.0:0")
        .expect("failed to bind to random port")
        .local_addr()
        .expect("failed to get local address")
        .port()
}
