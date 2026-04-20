# discord-proxy-rs

[**На русском языке**](README_RU.md)

## Description
This project is a proxy DLL (`version.dll`) designed for the Discord desktop client. It acts as a bridge between the Electron-based client and local proxy tools (Clash, V2Ray, etc.). It enables full proxy support for Discord, which lacks native comprehensive proxy configuration options.

## Purpose
The tool intercepts process initialization and network calls to inject proxy settings via dynamic PAC (Proxy Auto-Config) scripts. It ensures that both the main process and renderer processes route their traffic through specified SOCKS5 or HTTP tunnels.

## Why Rust Reimplementation
- **Memory Safety**: Eliminates common memory-related vulnerabilities and crashes inherent in C++/Delphi hooks.
- **Modern Ecosystem**: Provides access to high-performance libraries for detouring and binary manipulation.
- **Developer Experience**: Offers a cleaner, maintainable codebase for current systems programming standards.
- **Zero Overhead**: Minimal impact on process startup time and runtime resource consumption.
- **Robustness**: Improved handling of multi-process Electron architecture and race conditions during hook installation.

## Advantages of PAC-based Implementation
Compared to traditional socket-level hooking (often used in earlier Pascal/C++ implementations), this PAC-based approach offers:
- **Native Efficiency**: Uses Chromium's built-in proxy engine, ensuring 100% compatibility with all internal Electron requests (HTTP, WebSockets, etc.).
- **SSL/TLS Integrity**: Traffic is proxied at the application level, avoiding complex and unstable low-level interceptors that often interfere with encrypted connections.
- **Protocol Flexibility**: Supports complex routing logic through JavaScript-based PAC scripts without rewriting binary hook logic.
- **Stability**: Significantly reduces the risk of crashes and conflicts with antivirus software or other system-level network tools.

## Installation
1. Compile the project using `cargo build`.
2. Copy the resulting `version.dll` and the `drover.toml` configuration file to the following directory:
   `%localappdata%/Discord/app-<version>/` (e.g., `app-1.0.9147`).
3. Ensure the address and port in `drover.toml` match your local proxy server (Clash, V2Ray, etc.).
4. Restart Discord.

## Roadmap
- **Autonomous Operation**: Implementation of DPI bypass techniques based on the GoodbyeDPI logic to allow direct connection without external proxy clients.
- **Advanced UDP Fragmentation**: Enhanced packet splitting for voice chat stability under heavy DPI.
- **DNS-over-HTTPS (DoH)**: Integrated secure DNS resolution to prevent provider-side domain poisoning.
- **Certificate Handling**: Automated management of local certificates for internal traffic inspection.

## Configuration
Settings are managed via `drover.toml`. 
- `main`: Primary SOCKS5/HTTP proxy address.
- `rules`: Domain-specific routing overrides.
