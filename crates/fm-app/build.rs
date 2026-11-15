use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=vagnarok.manifest");
    println!("cargo:rerun-if-changed=vagnarok.rc");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let manifest = manifest_dir.join("vagnarok.manifest");

    if target_env == "msvc" {
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
            manifest.display()
        );
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());
    let rc_obj = out_dir.join("vagnarok_res.o");
    let rc_src = manifest_dir.join("vagnarok.rc");

    for tool in ["x86_64-w64-mingw32-windres", "windres"] {
        let status = Command::new(tool)
            .args([
                rc_src.to_string_lossy().as_ref(),
                "-O",
                "coff",
                "-o",
                rc_obj.to_string_lossy().as_ref(),
            ])
            .status();
        if let Ok(st) = status
            && st.success()
        {
            println!("cargo:rustc-link-arg-bins={}", rc_obj.display());
            break;
        }
    }
}
