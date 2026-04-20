# discord-proxy-rs

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

## Roadmap
- **Autonomous Operation**: Implementation of DPI bypass techniques based on the GoodbyeDPI logic to allow direct connection without external proxy clients.
- **Advanced UDP Fragmentation**: Enhanced packet splitting for voice chat stability under heavy DPI.
- **DNS-over-HTTPS (DoH)**: Integrated secure DNS resolution to prevent provider-side domain poisoning.
- **Certificate Handling**: Automated management of local certificates for internal traffic inspection.

## Configuration
Settings are managed via `drover.toml`. 
- `main`: Primary SOCKS5/HTTP proxy address.
- `rules`: Domain-specific routing overrides.
