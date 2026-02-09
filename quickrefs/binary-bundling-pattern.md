# Binary Bundling Pattern (Tauri v2)

Goal: include a companion Rust binary in the macOS app bundle at:
`Tairseach.app/Contents/MacOS/tairseach-mcp`

## Pattern Used

1. **Build the companion binary before app bundling**
   - `src-tauri/tauri.conf.json`:
   - `build.beforeBuildCommand = "npm run build && cargo build -p tairseach-mcp --release"`

2. **Register as a Tauri external binary**
   - `src-tauri/tauri.conf.json`:
   - `bundle.externalBin = ["binaries/tairseach-mcp"]`

3. **Prepare target-suffixed sidecar name in build script**
   - `src-tauri/build.rs` copies:
   - `../target/release/tairseach-mcp`
   - to:
   - `src-tauri/binaries/tairseach-mcp-<target-triple>`

Tauri expects `externalBin` artifacts with target suffixes (`-aarch64-apple-darwin`, etc.).
At bundle time it renames/places it into `Contents/MacOS/tairseach-mcp`.

## Notes

- `build.rs` emits a warning (does not fail) if `tairseach-mcp` isn’t built yet.
- This keeps `cargo check` fast/safe while ensuring `cargo tauri build` has the binary ready.
