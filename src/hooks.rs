use retour::static_detour;
use windows::{
    Win32::{
        Networking::WinSock::{SOCKADDR, SOCKET, WSABUF, WSAPROTOCOL_INFOA, WSAPROTOCOL_INFOW},
        System::LibraryLoader::{GetProcAddress, LoadLibraryA},
    },
    core::s,
};

use crate::{state, tcp_proxy, udp_bypass};

static_detour! {
    pub static HookSocket: unsafe extern "system" fn(i32, i32, i32) -> SOCKET;
    pub static HookWsaSocketW: unsafe extern "system" fn(i32, i32, i32, *const WSAPROTOCOL_INFOW, u32, u32) -> SOCKET;
    pub static HookWsaSocketA: unsafe extern "system" fn(i32, i32, i32, *const WSAPROTOCOL_INFOA, u32, u32) -> SOCKET;
    pub static HookCloseSocket: unsafe extern "system" fn(SOCKET) -> i32;
    pub static HookSend: unsafe extern "system" fn(SOCKET, *const u8, i32, i32) -> i32;
    pub static HookRecv: unsafe extern "system" fn(SOCKET, *mut u8, i32, i32) -> i32;
    pub static HookWsaSend: unsafe extern "system" fn(SOCKET, *const WSABUF, u32, *mut u32, u32, *mut std::ffi::c_void, *mut std::ffi::c_void) -> i32;
    pub static HookSendTo: unsafe extern "system" fn(SOCKET, *const u8, i32, i32, *const SOCKADDR, i32) -> i32;
    pub static HookWsaSendTo: unsafe extern "system" fn(SOCKET, *const WSABUF, u32, *mut u32, u32, *const SOCKADDR, i32, *mut std::ffi::c_void, *mut std::ffi::c_void) -> i32;
}

unsafe extern "system" fn my_socket(af: i32, type_: i32, protocol: i32) -> SOCKET {
    let sock = unsafe { HookSocket.call(af, type_, protocol) };
    crate::log!("Socket created: af={}, type={}, proto={}, sock={}", af, type_, protocol, sock.0);
    state::register_socket(sock.0, type_);
    sock
}

unsafe extern "system" fn my_wsa_socket_w(
    af: i32,
    type_: i32,
    protocol: i32,
    lp_protocol_info: *const WSAPROTOCOL_INFOW,
    g: u32,
    dw_flags: u32,
) -> SOCKET {
    let sock = unsafe { HookWsaSocketW.call(af, type_, protocol, lp_protocol_info, g, dw_flags) };
    crate::log!("WSASocketW created: sock={}", sock.0);
    state::register_socket(sock.0, type_);
    sock
}

unsafe extern "system" fn my_wsa_socket_a(
    af: i32,
    type_: i32,
    protocol: i32,
    lp_protocol_info: *const WSAPROTOCOL_INFOA,
    g: u32,
    dw_flags: u32,
) -> SOCKET {
    let sock = unsafe { HookWsaSocketA.call(af, type_, protocol, lp_protocol_info, g, dw_flags) };
    crate::log!("WSASocketA created: sock={}", sock.0);
    state::register_socket(sock.0, type_);
    sock
}

unsafe extern "system" fn my_closesocket(s: SOCKET) -> i32 {
    crate::log!("Socket closed: sock={}", s.0);
    state::remove_socket(s.0);
    unsafe { HookCloseSocket.call(s) }
}

unsafe extern "system" fn my_send(sock: SOCKET, buf: *const u8, len: i32, flags: i32) -> i32 {
    if let Some(mut st) = state::SOCKETS.get_mut(&sock.0) {
        if !st.first_send_done {
            st.first_send_done = true;
            if !buf.is_null() && len > 0 {
                let slice = unsafe { std::slice::from_raw_parts(buf, len as usize) };
                if tcp_proxy::convert_http_to_socks5(&mut st, sock, slice, flags) {
                    crate::log!("HTTP to SOCKS5 conversion triggered for sock={}", sock.0);
                    return len;
                }
            }
        }
    }
    unsafe { HookSend.call(sock, buf, len, flags) }
}

unsafe extern "system" fn my_wsa_send(
    sock: SOCKET,
    lp_buffers: *const WSABUF,
    dw_buffer_count: u32,
    lp_number_of_bytes_sent: *mut u32,
    dw_flags: u32,
    lp_overlapped: *mut std::ffi::c_void,
    lp_completion_routine: *mut std::ffi::c_void,
) -> i32 {
    let mut handled = false;
    let mut total_len = 0;

    if let Some(mut st) = state::SOCKETS.get_mut(&sock.0) {
        if !st.first_send_done {
            st.first_send_done = true;
            if dw_buffer_count == 1 && !lp_buffers.is_null() {
                let buf_info = unsafe { *lp_buffers };
                if buf_info.len > 0 && !buf_info.buf.is_null() {
                    let slice = unsafe {
                        std::slice::from_raw_parts(
                            buf_info.buf.0 as *const u8,
                            buf_info.len as usize,
                        )
                    };
                    if tcp_proxy::convert_http_to_socks5(&mut st, sock, slice, 0) {
                        crate::log!(
                            "WSASend HTTP to SOCKS5 conversion triggered for sock={}",
                            sock.0
                        );
                        handled = true;
                        total_len = buf_info.len;
                    }
                }
            }
        }
    }

    if handled {
        if !lp_number_of_bytes_sent.is_null() {
            unsafe { *lp_number_of_bytes_sent = total_len };
        }
        return 0; // 0 = standard success in WSASend
    }

    unsafe {
        HookWsaSend.call(
            sock,
            lp_buffers,
            dw_buffer_count,
            lp_number_of_bytes_sent,
            dw_flags,
            lp_overlapped,
            lp_completion_routine,
        )
    }
}

unsafe extern "system" fn my_recv(sock: SOCKET, buf: *mut u8, len: i32, flags: i32) -> i32 {
    let res = unsafe { HookRecv.call(sock, buf, len, flags) };
    if res > 0 && !buf.is_null() {
        if let Some(mut st) = state::SOCKETS.get_mut(&sock.0) {
            return tcp_proxy::handle_fake_http_response(&mut st, buf, len, res);
        }
    }
    res
}

unsafe extern "system" fn my_sendto(
    sock: SOCKET,
    buf: *const u8,
    len: i32,
    flags: i32,
    to: *const SOCKADDR,
    tolen: i32,
) -> i32 {
    if let Some(mut st) = state::SOCKETS.get_mut(&sock.0) {
        if !st.first_send_done {
            st.first_send_done = true;
            udp_bypass::check_and_split_udp_sendto(&mut st, sock, len, to, tolen);
        }
    }
    unsafe { HookSendTo.call(sock, buf, len, flags, to, tolen) }
}

unsafe extern "system" fn my_wsa_sendto(
    sock: SOCKET,
    lp_buffers: *const WSABUF,
    dw_buffer_count: u32,
    lp_number_of_bytes_sent: *mut u32,
    dw_flags: u32,
    lp_to: *const SOCKADDR,
    tolen: i32,
    lp_overlapped: *mut std::ffi::c_void,
    lp_completion_routine: *mut std::ffi::c_void,
) -> i32 {
    if let Some(mut st) = state::SOCKETS.get_mut(&sock.0) {
        if !st.first_send_done {
            st.first_send_done = true;
            udp_bypass::check_and_split_wsa_sendto(
                &mut st,
                sock,
                lp_buffers,
                dw_buffer_count,
                lp_to,
                tolen,
            );
        }
    }
    unsafe {
        HookWsaSendTo.call(
            sock,
            lp_buffers,
            dw_buffer_count,
            lp_number_of_bytes_sent,
            dw_flags,
            lp_to,
            tolen,
            lp_overlapped,
            lp_completion_routine,
        )
    }
}

pub unsafe fn setup_socket_hooks() {
    unsafe {
        crate::log!("Loading ws2_32.dll for hooks...");
        if let Ok(ws2_32) = LoadLibraryA(s!("ws2_32.dll")) {
            if let Some(addr) = GetProcAddress(ws2_32, s!("socket")) {
                let original: unsafe extern "system" fn(i32, i32, i32) -> SOCKET =
                    std::mem::transmute(addr);
                let _ = HookSocket
                    .initialize(original, |af, t, p| my_socket(af, t, p))
                    .unwrap()
                    .enable();
                crate::log!("socket hooked.");
            }

            if let Some(addr) = GetProcAddress(ws2_32, s!("WSASocketW")) {
                let original: unsafe extern "system" fn(
                    i32,
                    i32,
                    i32,
                    *const WSAPROTOCOL_INFOW,
                    u32,
                    u32,
                ) -> SOCKET = std::mem::transmute(addr);
                let _ = HookWsaSocketW
                    .initialize(original, |af, t, p, i, g, f| my_wsa_socket_w(af, t, p, i, g, f))
                    .unwrap()
                    .enable();
                crate::log!("WSASocketW hooked.");
            }

            if let Some(addr) = GetProcAddress(ws2_32, s!("WSASocketA")) {
                let original: unsafe extern "system" fn(
                    i32,
                    i32,
                    i32,
                    *const WSAPROTOCOL_INFOA,
                    u32,
                    u32,
                ) -> SOCKET = std::mem::transmute(addr);
                let _ = HookWsaSocketA
                    .initialize(original, |af, t, p, i, g, f| my_wsa_socket_a(af, t, p, i, g, f))
                    .unwrap()
                    .enable();
                crate::log!("WSASocketA hooked.");
            }

            if let Some(addr) = GetProcAddress(ws2_32, s!("closesocket")) {
                let original: unsafe extern "system" fn(SOCKET) -> i32 = std::mem::transmute(addr);
                let _ =
                    HookCloseSocket.initialize(original, |s| my_closesocket(s)).unwrap().enable();
                crate::log!("closesocket hooked.");
            }

            if let Some(addr) = GetProcAddress(ws2_32, s!("send")) {
                let original: unsafe extern "system" fn(SOCKET, *const u8, i32, i32) -> i32 =
                    std::mem::transmute(addr);
                let _ = HookSend
                    .initialize(original, |s, b, l, f| my_send(s, b, l, f))
                    .unwrap()
                    .enable();
                crate::log!("send hooked.");
            }

            if let Some(addr) = GetProcAddress(ws2_32, s!("WSASend")) {
                let original: unsafe extern "system" fn(
                    SOCKET,
                    *const WSABUF,
                    u32,
                    *mut u32,
                    u32,
                    *mut std::ffi::c_void,
                    *mut std::ffi::c_void,
                ) -> i32 = std::mem::transmute(addr);
                let _ = HookWsaSend
                    .initialize(original, |s, b, c, n, f, o, r| my_wsa_send(s, b, c, n, f, o, r))
                    .unwrap()
                    .enable();
                crate::log!("WSASend hooked.");
            }

            if let Some(addr) = GetProcAddress(ws2_32, s!("recv")) {
                let original: unsafe extern "system" fn(SOCKET, *mut u8, i32, i32) -> i32 =
                    std::mem::transmute(addr);
                let _ = HookRecv
                    .initialize(original, |s, b, l, f| my_recv(s, b, l, f))
                    .unwrap()
                    .enable();
                crate::log!("recv hooked.");
            }

            if let Some(addr) = GetProcAddress(ws2_32, s!("sendto")) {
                let original: unsafe extern "system" fn(
                    SOCKET,
                    *const u8,
                    i32,
                    i32,
                    *const SOCKADDR,
                    i32,
                ) -> i32 = std::mem::transmute(addr);
                let _ = HookSendTo
                    .initialize(original, |s, b, l, f, t, tl| my_sendto(s, b, l, f, t, tl))
                    .unwrap()
                    .enable();
                crate::log!("sendto hooked.");
            }

            if let Some(addr) = GetProcAddress(ws2_32, s!("WSASendTo")) {
                let original: unsafe extern "system" fn(
                    SOCKET,
                    *const WSABUF,
                    u32,
                    *mut u32,
                    u32,
                    *const SOCKADDR,
                    i32,
                    *mut std::ffi::c_void,
                    *mut std::ffi::c_void,
                ) -> i32 = std::mem::transmute(addr);
                let _ = HookWsaSendTo
                    .initialize(original, |s, b, c, n, f, t, tl, o, r| {
                        my_wsa_sendto(s, b, c, n, f, t, tl, o, r)
                    })
                    .unwrap()
                    .enable();
                crate::log!("WSASendTo hooked.");
            }
        }
    }
}
