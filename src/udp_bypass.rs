use windows::Win32::Networking::WinSock::{SOCKADDR, SOCKET, WSABUF};

use crate::{hooks::HookSendTo, state::SocketState};

const DISCORD_VOICE_PACKET_LEN: i32 = 74;
const DISCORD_VOICE_PACKET_LEN_U32: u32 = 74;
const SPLIT_DELAY_MS: u64 = 50;

pub fn check_and_split_udp_sendto(
    state: &mut SocketState,
    sock: SOCKET,
    len: i32,
    to: *const SOCKADDR,
    tolen: i32,
) {
    if !state.is_udp || len != DISCORD_VOICE_PACKET_LEN {
        return;
    }

    let dummy_packet_0: [u8; 1] = [0];
    let dummy_packet_1: [u8; 1] = [1];

    unsafe {
        HookSendTo.call(sock, dummy_packet_0.as_ptr(), dummy_packet_0.len() as i32, 0, to, tolen);
        HookSendTo.call(sock, dummy_packet_1.as_ptr(), dummy_packet_1.len() as i32, 0, to, tolen);
    }

    std::thread::sleep(std::time::Duration::from_millis(SPLIT_DELAY_MS));
}

pub fn check_and_split_wsa_sendto(
    state: &mut SocketState,
    sock: SOCKET,
    lp_buffers: *const WSABUF,
    dw_buffer_count: u32,
    to: *const SOCKADDR,
    tolen: i32,
) {
    if !state.is_udp || dw_buffer_count != 1 {
        return;
    }

    let buf_info = unsafe { *lp_buffers };
    if buf_info.len != DISCORD_VOICE_PACKET_LEN_U32 {
        return;
    }

    let dummy_packet_0: [u8; 1] = [0];
    let dummy_packet_1: [u8; 1] = [1];

    unsafe {
        // Fallback to basic sendto for fragmentation, even if original is WSASendTo
        HookSendTo.call(sock, dummy_packet_0.as_ptr(), dummy_packet_0.len() as i32, 0, to, tolen);
        HookSendTo.call(sock, dummy_packet_1.as_ptr(), dummy_packet_1.len() as i32, 0, to, tolen);
    }

    std::thread::sleep(std::time::Duration::from_millis(SPLIT_DELAY_MS));
}
