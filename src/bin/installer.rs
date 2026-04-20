use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

use colored::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Discord Proxy Installer & Configurator");

    // 1. Path Setup
    let current_dir = env::current_dir()?;
    let source_dll = current_dir.join("version.dll");
    let source_config = current_dir.join("drover.toml");

    if !source_dll.exists() {
        println!("{} {}", "error:".red().bold(), "'version.dll' not found in current directory!");
        println!("Please ensure version.dll is in the same folder as this installer.");
        wait_exit();
        return Ok(());
    }

    // 2. Interactive Config Setup
    println!("\n{} Configuration Setup", "»".blue().bold());
    println!("Press Enter to skip if drover.toml is already configured.");
    print!("Proxy address (e.g. {}): ", "socks5://127.0.0.1:10808".dimmed());
    io::stdout().flush()?;

    let mut proxy_input = String::new();
    io::stdin().read_line(&mut proxy_input)?;
    let proxy_input = proxy_input.trim();

    if !proxy_input.is_empty() {
        let config_content =
            format!("[proxy]\nmain = \"{}\"\nfallback = \"http://127.0.0.1:10808\"\n", proxy_input);
        fs::write(&source_config, config_content)?;
        println!(" {} Configuration saved.", "✓".green());
    }

    // 3. Locate Discord
    println!("\n{} Locating Discord", "»".blue().bold());
    let local_app_data = env::var("LOCALAPPDATA")?;
    let discord_root = Path::new(&local_app_data).join("Discord");

    if !discord_root.exists() {
        println!("{} Discord folder not found in %LOCALAPPDATA%!", "error:".red().bold());
        wait_exit();
        return Ok(());
    }

    let entries = fs::read_dir(&discord_root)?;
    let app_dirs: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path.file_name().unwrap_or_default().to_string_lossy().starts_with("app-")
        })
        .collect();

    if app_dirs.is_empty() {
        println!("{} No app-* versions found in Discord directory.", "error:".red().bold());
        wait_exit();
        return Ok(());
    }

    let mut latest_app_dir = app_dirs[0].clone();
    for dir in app_dirs.iter().skip(1) {
        if dir > &latest_app_dir {
            latest_app_dir = dir.clone();
        }
    }

    println!(" {} Target: {}", "→".cyan(), latest_app_dir.display().to_string().dimmed());

    // 4. Copy Files
    println!("\n{} Installing Components", "»".blue().bold());
    let target_dll = latest_app_dir.join("version.dll");
    let target_config = latest_app_dir.join("drover.toml");

    fs::copy(&source_dll, &target_dll)?;
    println!(" {} {} installed.", "✓".green(), "version.dll");

    if source_config.exists() {
        fs::copy(&source_config, &target_config)?;
        println!(" {} {} installed.", "✓".green(), "drover.toml");
    }

    // 5. Restart Prompt
    println!("\n{}", "------------------------------------------------".blue());
    println!("{}", "Installation completed successfully!".green().bold());

    print!("\n{} (y/n): ", "Restart Discord now?".cyan());
    io::stdout().flush()?;

    let mut restart_input = String::new();
    io::stdin().read_line(&mut restart_input)?;
    if restart_input.trim().to_lowercase() == "y" {
        restart_discord(&latest_app_dir);
    } else {
        println!("Changes will take effect after a manual restart.");
    }

    wait_exit();
    Ok(())
}

fn restart_discord(app_dir: &Path) {
    use std::{os::windows::process::CommandExt, process::Stdio};
    const DETACHED_PROCESS: u32 = 0x00000008;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const FLAGS: u32 = DETACHED_PROCESS | CREATE_NO_WINDOW;

    print!("Restarting processes...");
    io::stdout().flush().unwrap();

    let _ = Command::new("taskkill").args(&["/F", "/IM", "Discord.exe"]).output();

    std::thread::sleep(std::time::Duration::from_secs(1));

    let discord_exe = app_dir.join("Discord.exe");
    if discord_exe.exists() {
        let status = Command::new(discord_exe)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(FLAGS)
            .spawn();

        if status.is_ok() {
            println!(" {}", "done.".green());
        }
    } else {
        let root_dir = app_dir.parent().unwrap();
        let update_exe = root_dir.join("Update.exe");
        if update_exe.exists() {
            let _ = Command::new(update_exe)
                .args(&["--processStart", "Discord.exe"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(FLAGS)
                .spawn();
            println!(" {}", "done (via Update.exe).".green());
        }
    }
}

fn wait_exit() {
    println!("\nPress Enter to exit...");
    let _ = io::stdin().read_line(&mut String::new());
}
