use once_cell::sync::Lazy;
use regex::bytes::Regex;
use windows::Win32::Networking::WinSock::{FD_SET, SOCKET, TIMEVAL, select};

use crate::{
    hooks::{HookRecv, HookSend},
    state::SocketState,
};

pub fn convert_http_to_socks5(
    state: &mut SocketState,
    sock: SOCKET,
    buf: &[u8],
    flags: i32,
) -> bool {
    if !state.is_tcp {
        return false;
    }

    let config = &*crate::config::CONFIG;
    let Some(main_proxy) = &config.proxy.main else {
        return false;
    };
    if main_proxy.is_empty() || !main_proxy.starts_with("socks5://") {
        return false;
    }

    if buf.len() < 8 || &buf[0..8] != b"CONNECT " {
        return false;
    }

    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\ACONNECT ([a-z0-9.-]+):(\d+)").unwrap());

    let Some(caps) = RE.captures(buf) else {
        return false;
    };

    let target_host = caps.get(1).unwrap().as_bytes();
    let Ok(target_port_str) = std::str::from_utf8(caps.get(2).unwrap().as_bytes()) else {
        return false;
    };
    let Ok(target_port) = target_port_str.parse::<u16>() else {
        return false;
    };

    // SOCKS5 Greeting: 0x05, 0x01, 0x00
    let greeting = [5u8, 1, 0];
    unsafe {
        if HookSend.call(sock, greeting.as_ptr(), greeting.len() as i32, flags)
            != greeting.len() as i32
        {
            return false;
        }
    }

    // Wait for response via select
    let mut fd_set = FD_SET { fd_count: 1, fd_array: [sock; 64] };
    let tv = TIMEVAL { tv_sec: 10, tv_usec: 0 };
    unsafe {
        if select(0, Some(&mut fd_set as *mut _), None, None, Some(&tv as *const _)) < 1 {
            return false;
        }
    }

    let mut resp = [0u8; 2];
    unsafe {
        if HookRecv.call(sock, resp.as_mut_ptr(), 2, 0) != 2 {
            return false;
        }
    }

    if resp != [5, 0] {
        return false;
    }

    // Send Request: 0x05, 0x01, 0x00, 0x03, Host len, Host, Port Hi, Port Lo
    let mut req = Vec::with_capacity(7 + target_host.len());
    req.extend_from_slice(&[5, 1, 0, 3, target_host.len() as u8]);
    req.extend_from_slice(target_host);
    req.extend_from_slice(&target_port.to_be_bytes());

    unsafe {
        if HookSend.call(sock, req.as_ptr(), req.len() as i32, flags) != req.len() as i32 {
            return false;
        }
    }

    state.fake_http_proxy = true;
    true
}

pub fn handle_fake_http_response(state: &mut SocketState, buf: *mut u8, len: i32, res: i32) -> i32 {
    if state.fake_http_proxy {
        state.fake_http_proxy = false; // Reset flag after one use
        if res >= 10 {
            let slice = unsafe { std::slice::from_raw_parts(buf, res as usize) };
            // If server replied with SOCKS5 connection success
            if slice.len() >= 3 && slice[0] == 5 && slice[1] == 0 && slice[2] == 0 {
                let fake_resp = b"HTTP/1.1 200 Connection Established\r\n\r\n";
                if fake_resp.len() as i32 <= len {
                    unsafe {
                        std::ptr::copy_nonoverlapping(fake_resp.as_ptr(), buf, fake_resp.len());
                    }
                    return fake_resp.len() as i32;
                }
            }
        }
    }
    res
}
