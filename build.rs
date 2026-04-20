fn main() {
    let functions = [
        "GetFileVersionInfoA",
        "GetFileVersionInfoExA",
        "GetFileVersionInfoExW",
        "GetFileVersionInfoSizeA",
        "GetFileVersionInfoSizeExA",
        "GetFileVersionInfoSizeExW",
        "GetFileVersionInfoSizeW",
        "GetFileVersionInfoW",
        "VerFindFileA",
        "VerFindFileW",
        "VerInstallFileA",
        "VerInstallFileW",
        "VerLanguageNameA",
        "VerLanguageNameW",
        "VerQueryValueA",
        "VerQueryValueW",
    ];

    for func in functions {
        println!(
            "cargo:rustc-link-arg-cdylib=/EXPORT:{}={}.{}",
            func, "C:\\Windows\\System32\\version", func
        );
    }
}
