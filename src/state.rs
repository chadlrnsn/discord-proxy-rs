use dashmap::DashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy)]
pub struct SocketState {
    pub is_tcp: bool,
    pub is_udp: bool,
    pub first_send_done: bool,
    pub fake_http_proxy: bool,
}

pub static SOCKETS: Lazy<DashMap<usize, SocketState>> = Lazy::new(|| DashMap::new());

pub fn register_socket(sock_id: usize, type_: i32) {
    if sock_id != !0 {
        // SOCK_STREAM = 1, SOCK_DGRAM = 2
        SOCKETS.insert(
            sock_id,
            SocketState {
                is_tcp: type_ == 1,
                is_udp: type_ == 2,
                first_send_done: false,
                fake_http_proxy: false,
            },
        );
    }
}

pub fn remove_socket(sock_id: usize) {
    SOCKETS.remove(&sock_id);
}
