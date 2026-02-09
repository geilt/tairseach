use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Prepare sidecar binary for bundling:
    // src-tauri/binaries/tairseach-mcp-<target-triple>
    let target = match env::var("TARGET") {
        Ok(t) => t,
        Err(_) => return,
    };

    let binaries_dir = PathBuf::from("binaries");
    if let Err(e) = fs::create_dir_all(&binaries_dir) {
        println!("cargo:warning=failed to create binaries dir: {e}");
        return;
    }

    let dest = binaries_dir.join(format!("tairseach-mcp-{target}"));

    let release_source = PathBuf::from("../target/release/tairseach-mcp");
    let debug_source = PathBuf::from("../target/debug/tairseach-mcp");

    if release_source.exists() {
        if let Err(e) = fs::copy(&release_source, &dest) {
            println!(
                "cargo:warning=failed to copy {} to {}: {e}",
                release_source.display(),
                dest.display()
            );
            return;
        }
    } else if debug_source.exists() {
        if let Err(e) = fs::copy(&debug_source, &dest) {
            println!(
                "cargo:warning=failed to copy {} to {}: {e}",
                debug_source.display(),
                dest.display()
            );
            return;
        }
    } else {
        println!(
            "cargo:warning=tairseach-mcp binary not found yet, creating placeholder sidecar at {}",
            dest.display()
        );
        let placeholder = "#!/bin/sh\necho 'tairseach-mcp sidecar not built'\nexit 1\n";
        if let Err(e) = fs::write(&dest, placeholder) {
            println!("cargo:warning=failed to create placeholder sidecar: {e}");
            return;
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&dest) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&dest, perms);
        }
    }

    // Note: We do NOT use cargo:rerun-if-changed for the sidecar binaries.
    // The sidecar is built explicitly via beforeBuildCommand in tauri.conf.json.
    // Watching the binary would create a rebuild loop since we copy it during build.

    // Keep existing Tauri build behavior.
    tauri_build::build();
}
