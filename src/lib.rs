mod config;
mod hooks;
mod state;
mod tcp_proxy;
mod udp_bypass;

use std::{fs, os::windows::ffi::OsStrExt};

use base64::Engine;
use once_cell::sync::Lazy;
use retour::static_detour;
#[cfg(debug_assertions)]
use windows::Win32::System::Console::{AllocConsole, SetConsoleTitleW};
#[cfg(debug_assertions)]
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::{
    Win32::{
        Foundation::HINSTANCE,
        System::{
            LibraryLoader::{GetModuleHandleA, GetProcAddress},
            SystemServices::DLL_PROCESS_ATTACH,
            Threading::{CreateThread, THREAD_CREATION_FLAGS},
        },
    },
    core::s,
};

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            #[allow(unused_unsafe)]
            let pid = unsafe { windows::Win32::System::Threading::GetCurrentProcessId() };
            let s = format!($($arg)*);
            let s = format!("[Proxy:{}] {}\n", pid, s);
            #[allow(unused_unsafe)]
            unsafe {
                use windows::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE, WriteConsoleA};
                if let Ok(handle) = GetStdHandle(STD_OUTPUT_HANDLE) {
                    let _ = WriteConsoleA(handle, s.as_bytes(), None, None);
                }
            }
        }
    };
}

static_detour! {
    static HookGetCommandLineW: unsafe extern "system" fn() -> *mut u16;
}

static NEW_COMMAND_LINE: Lazy<Vec<u16>> = Lazy::new(|| {
    log!("Generating new command line...");
    let mut cmd_line = String::new();

    unsafe {
        let ptr = HookGetCommandLineW.call();
        if !ptr.is_null() {
            let mut len = 0;
            while *ptr.offset(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr, len as usize);
            cmd_line = String::from_utf16_lossy(slice);
        }
    }

    let pac_script = if let Ok(content) = fs::read_to_string("drover.toml") {
        let conf: config::Config = toml::from_str(&content).unwrap_or_default();
        config::generate_pac_script(&conf)
    } else {
        config::generate_pac_script(&config::Config::default())
    };

    if !cmd_line.contains("--proxy-pac-url") {
        let base64_pac = base64::engine::general_purpose::STANDARD.encode(&pac_script);
        log!(
            "PAC script preview: {}...",
            &pac_script[..pac_script.len().min(100)].replace("\n", " ")
        );
        let append = format!(
            " --proxy-pac-url=\"data:application/x-ns-proxy-autoconfig;base64,{}\"",
            base64_pac
        );
        cmd_line.push_str(&append);
    } else {
        log!("Command line already contains proxy settings.");
    }
    log!("New Command Line length: {}", cmd_line.len());

    let mut wide: Vec<u16> = std::ffi::OsStr::new(&cmd_line).encode_wide().collect();
    wide.push(0);
    wide
});

unsafe extern "system" fn my_get_command_line_w() -> *mut u16 {
    NEW_COMMAND_LINE.as_ptr() as *mut u16
}

unsafe extern "system" fn init_thread(_param: *mut std::ffi::c_void) -> u32 {
    log!("Initialization thread started.");

    // Wait for kernel32.dll just in case, though it's always there
    while unsafe { GetModuleHandleA(s!("kernel32.dll")) }.is_err() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    log!("Kernel32 found, setting up hooks...");
    unsafe { setup_hooks() };

    // Wait for ws2_32.dll if we want to be reactive,
    // but hooks::setup_socket_hooks already uses LoadLibraryA which is fine.
    unsafe { hooks::setup_socket_hooks() };

    log!("Hooks initialized successfully.");
    0
}

unsafe fn setup_hooks() {
    unsafe {
        if let Ok(kernel32) = GetModuleHandleA(s!("kernel32.dll")) {
            let get_cmd = GetProcAddress(kernel32, s!("GetCommandLineW"));
            if let Some(addr) = get_cmd {
                let target: unsafe extern "system" fn() -> *mut u16 = std::mem::transmute(addr);
                let _ = HookGetCommandLineW
                    .initialize(target, || my_get_command_line_w())
                    .unwrap()
                    .enable();
                log!("GetCommandLineW hooked.");
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(
    _hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *mut std::ffi::c_void,
) -> i32 {
    if fdw_reason == DLL_PROCESS_ATTACH {
        #[cfg(debug_assertions)]
        unsafe {
            // Check if we are in the main process (not a renderer/utility)
            let cmd = windows::Win32::System::Environment::GetCommandLineW();
            let mut is_main = true;
            if !cmd.is_null() {
                let mut len = 0;
                while *cmd.0.offset(len) != 0 && len < 512 {
                    len += 1;
                }
                let s = String::from_utf16_lossy(std::slice::from_raw_parts(cmd.0, len as usize));
                if s.contains("--type=") {
                    is_main = false;
                }
            }

            if is_main {
                let _ = AllocConsole();
                let pid = GetCurrentProcessId();
                let title = format!("Discord Proxy Main [PID:{}]", pid);
                let _ = SetConsoleTitleW(&windows::core::HSTRING::from(title));
                log!("Debug console allocated for main process.");
            }
        }

        unsafe {
            // Spawn initialization thread to avoid Loader Lock
            let _ = CreateThread(None, 0, Some(init_thread), None, THREAD_CREATION_FLAGS(0), None);
        }
    }
    1
}
