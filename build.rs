use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn main() {
    if cfg!(target_env = "msvc") {
        println!("cargo:rustc-link-arg=-static");
        println!("cargo:rustc-link-arg=-C target-feature=+crt-static");
    }
    
    println!("cargo:rustc-link-arg-bin=minigrep=app_icon.res");

    if cfg!(target_env = "gnu") {
        println!("cargo:rustc-link-arg=-static");
    }

    let rust_version = Command::new("rustc")
        .arg("--version")
        .output()
        .expect("Failed to get rust version")
        .stdout;

    // let version = env::var("CARGO_PKG_VERSION").unwrap().to_string();


    let rust_version = String::from_utf8_lossy(&rust_version);
    
    // 提取 Rust 版本号（例如: "rustc 1.75.0 (8b8d65e3d 2024-07-12)"）
    let version_parts: Vec<&str> = rust_version.split_whitespace().collect();
    let rustc_version = version_parts[1];  // 1.75.0

    // 获取 Cargo.toml 中的版本号
    let package_version = env::var("CARGO_PKG_VERSION").unwrap(); // 获取包的版本号，像 "1.6.1"

    // 获取目标平台
    let target = env::var("TARGET").unwrap();

    // 默认文件名
    let mut file_name = format!("minigrep-{}", package_version);

    // 检查 Rust 版本，如果是 1.75.0，则添加 "win7"
    if rustc_version == "1.75.0" {
        file_name.push_str("-win7");
    }

    // 根据平台添加标识（例如 "windows", "linux"）
    if target.contains("windows") {
        file_name.push_str("-windows");
    } else if target.contains("linux") {
        file_name.push_str("-linux");
    }

    // 如果是 x86_64 架构，添加 "-amd64"
    if target.contains("x86_64") {
        file_name.push_str("-amd64");
    }

    // 为文件名添加扩展名
    if target.contains("windows") {
        file_name.push_str(".exe"); // Windows 下为 .exe
    }

    let mut f = File::create(
        Path::new(&env::var("OUT_DIR").unwrap())
            .join("VERSION")).unwrap();
    f.write_all(file_name.trim().as_bytes()).unwrap();

    // println!("cargo:rerun-if-changed=build.rs");
    // let output_dir = "target/release";  
    // let old_file = format!("{}/minigrep.exe", output_dir);
    // let new_file = format!("{}/{}", output_dir, file_name);

    // std::fs::rename(old_file, new_file).expect("Failed to rename file");

}
