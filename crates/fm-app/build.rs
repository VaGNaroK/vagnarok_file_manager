use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=vagnarok.manifest");
    println!("cargo:rerun-if-changed=vagnarok.rc");
    println!("cargo:rerun-if-changed=../../assets/icons/vagnarok.ico");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let icon = manifest_dir.join("../../assets/icons/vagnarok.ico");
    let manifest = manifest_dir.join("vagnarok.manifest");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(icon.to_string_lossy().as_ref());
    res.set_manifest_file(manifest.to_string_lossy().as_ref());
    res.set("ProductName", "Vagnarok File Manager");
    res.set("FileDescription", "Native Cross-Platform Rust File Manager");
    if let Err(err) = res.compile() {
        println!("cargo:warning=failed to embed Windows resources: {err}");
    }
}
