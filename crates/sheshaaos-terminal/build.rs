use std::{env, path::PathBuf, process::Command};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let zig_src = manifest_dir.join("zig_src").join("vt100_expert.zig");

    let home = env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let zig_local = PathBuf::from(home).join(".local/bin/zig");
    let zig_bin = if zig_local.exists() { zig_local } else { PathBuf::from("zig") };

    let lib_output = out_dir.join("libvt100_expert.a");

    let status = Command::new(&zig_bin)
        .arg("build-lib")
        .arg("-O")
        .arg("ReleaseFast")
        .arg(format!("-femit-bin={}", lib_output.display()))
        .arg("-lc")
        .arg(&zig_src)
        .status();

    if let Ok(s) = status {
        if s.success() {
            println!("cargo:rustc-link-search=native={}", out_dir.display());
            println!("cargo:rustc-link-lib=static=vt100_expert");
        } else {
            panic!("Zig compilation failed");
        }
    } else {
        panic!("Failed to execute Zig compiler at {:?}", zig_bin);
    }

    println!("cargo:rerun-if-changed={}", zig_src.display());
}
