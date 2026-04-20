use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Discord Proxy Installer & Configurator ---");

    // 1. Get source directory (where the installer is running)
    let current_dir = env::current_dir()?;
    let source_dll = current_dir.join("version.dll");
    let source_config = current_dir.join("drover.toml");

    if !source_dll.exists() {
        println!("Error: version.dll not found in the current directory.");
        println!("Please place your compiled 'version.dll' next to this installer.");
        return Ok(());
    }

    // 2. Interactive Config Setup
    println!("\n[Config Setup]");
    print!("Enter your SOCKS5/HTTP proxy (e.g., socks5://127.0.0.1:10808) [leave empty to skip]: ");
    io::stdout().flush()?;

    let mut proxy_input = String::new();
    io::stdin().read_line(&mut proxy_input)?;
    let proxy_input = proxy_input.trim();

    if !proxy_input.is_empty() {
        let config_content = format!(
            "[proxy]\nmain = \"{}\"\nfallback = \"http://127.0.0.1:10808\"\n\n# Rules are now optional, global proxying is enabled by default\n",
            proxy_input
        );
        fs::write(&source_config, config_content)?;
        println!("Updated drover.toml with your proxy settings.");
    }

    // 3. Find Discord Path
    let local_app_data = env::var("LOCALAPPDATA")?;
    let discord_path = Path::new(&local_app_data).join("Discord");

    if !discord_path.exists() {
        return Err("Discord installation not found in %LOCALAPPDATA%".into());
    }

    // 4. Find the latest app-<version> folder
    let mut app_folders: Vec<PathBuf> = fs::read_dir(&discord_path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path.file_name().unwrap_or_default().to_string_lossy().starts_with("app-")
        })
        .collect();

    app_folders.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    let target_dir = match app_folders.first() {
        Some(dir) => dir,
        None => return Err("No app-<version> folders found in Discord directory".into()),
    };

    println!("\nTarget directory: {}", target_dir.display());

    // 5. Copying
    fs::copy(&source_dll, target_dir.join("version.dll"))?;
    println!("SUCCESS: Copied version.dll");

    if source_config.exists() {
        fs::copy(&source_config, target_dir.join("drover.toml"))?;
        println!("SUCCESS: Copied drover.toml");
    }

    println!("\nInstallation complete! Please restart Discord.");
    println!("Press Enter to exit...");
    let _ = io::stdin().read_line(&mut String::new());

    Ok(())
}
