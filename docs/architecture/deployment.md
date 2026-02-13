# Deployment Architecture

Tairseach is a Tauri 2 desktop application bundled as a macOS `.app` with embedded Rust backend, Vue 3 frontend, and two external Go/Rust binaries.

---

## Build Targets

**Primary:** macOS (Apple Silicon, arm64)  
**Bundle format:** `.app` (application bundle) + optional `.dmg` installer

---

## Build Process

### Prerequisites

- **Rust:** 1.70+ (stable)
- **Node.js:** 18+ (LTS)
- **Go:** 1.21+ (for `op-helper` binary)
- **Xcode Command Line Tools** (for code signing)

### Install dependencies

```bash
# Frontend dependencies
npm install

# Rust dependencies (automatic via Cargo)
cd src-tauri && cargo fetch
```

---

## Development Mode

**Command:**
```bash
cargo tauri dev
```
or
```bash
npm run tauri dev
```

**What happens:**

1. Tauri runs `beforeDevCommand` from `tauri.conf.json` → `npm run dev`
2. Vite starts dev server at `http://localhost:1420`
3. Tauri spawns webview window pointing to dev server
4. Rust backend runs in `src-tauri/target/debug/tairseach`
5. Hot module replacement (HMR) enabled for frontend changes
6. Rust changes require manual `cargo build` + restart

**External binaries in dev mode:**

Dev mode does NOT bundle external binaries. You must manually build and place them:

```bash
# Build MCP bridge (Rust)
cd src-tauri && cargo build -p tairseach-mcp --release
mkdir -p bin
cp target/release/tairseach-mcp bin/

# Build 1Password helper (Go)
cd src-tauri/helpers/onepassword
./build.sh
# Creates: src-tauri/bin/op-helper
```

Dev mode reads binaries from `src-tauri/bin/` (not bundled).

---

## Production Build

**Command:**
```bash
cargo tauri build
```
or
```bash
npm run app:build
```

**Build pipeline:**

1. **Frontend build:** Tauri runs `beforeBuildCommand`:
   - `npm run build` → Vue/Vite compiles to `dist/`
   - `cargo build -p tairseach-mcp --release` → builds MCP bridge

2. **Rust backend build:**
   - Compiles `src-tauri/` to release binary
   - Strips debug symbols (release profile)
   - Links against macOS frameworks

3. **External binaries:**
   - Tauri copies `externalBin` entries to bundle:
     - `src-tauri/binaries/tairseach-mcp` (Rust MCP server)
     - `src-tauri/binaries/op-helper` (Go 1Password client)

4. **Bundle packaging:**
   - Creates `.app` bundle at `src-tauri/target/release/bundle/macos/Tairseach.app`
   - Embeds frontend assets (`dist/`) into app bundle
   - Embeds Rust backend binary
   - Copies external binaries to `Tairseach.app/Contents/MacOS/`
   - Applies macOS entitlements (`entitlements.plist`)

5. **DMG creation (optional):**
   - Creates installer DMG with custom layout
   - App icon at (180, 170), Applications folder at (480, 170)
   - Window size: 660x400

**Output:**
- `src-tauri/target/release/bundle/macos/Tairseach.app` (runnable app)
- `src-tauri/target/release/bundle/dmg/Tairseach_0.1.0_aarch64.dmg` (installer)

---

## External Binaries

Tairseach bundles two helper binaries defined in `tauri.conf.json`:

```json
"externalBin": [
  "binaries/tairseach-mcp",
  "binaries/op-helper"
]
```

### 1. tairseach-mcp (Rust MCP Server)

**Purpose:** MCP (Model Context Protocol) bridge server  
**Source:** `src-tauri/Cargo.toml` workspace member  
**Build:**
```bash
cargo build -p tairseach-mcp --release
```
**Output:** `src-tauri/target/release/tairseach-mcp`  
**Bundle location:** `Tairseach.app/Contents/MacOS/tairseach-mcp`

**Runtime behavior:**
- Spawned by Tauri app on startup via `tauri::api::process::Command`
- Listens on Unix socket `~/.tairseach/tairseach.sock`
- Proxies MCP tool calls to OpenClaw gateway
- Auto-managed lifecycle (killed when app quits)

### 2. op-helper (Go 1Password Client)

**Purpose:** 1Password SDK wrapper for credential retrieval  
**Source:** `src-tauri/helpers/onepassword/main.go`  
**Build:**
```bash
cd src-tauri/helpers/onepassword
./build.sh
```
**Build script (`build.sh`):**
```bash
#!/bin/bash
set -e

echo "Building 1Password helper binary..."

# Initialize go.mod if missing
if [ ! -f go.mod ]; then
    go mod init tairseach-op-helper
    go get github.com/1password/onepassword-sdk-go@latest
fi

# Build for darwin/arm64
GOOS=darwin GOARCH=arm64 go build -o ../../bin/op-helper main.go
```

**Output:** `src-tauri/bin/op-helper`  
**Bundle location:** `Tairseach.app/Contents/MacOS/op-helper`

**Runtime behavior:**
- Invoked on-demand via Rust `std::process::Command`
- Reads 1Password credentials via SDK
- Outputs JSON to stdout
- Ephemeral process (one call per credential fetch)

---

## Bundle Structure

```
Tairseach.app/
├── Contents/
│   ├── Info.plist              # App metadata
│   ├── MacOS/
│   │   ├── Tairseach           # Rust backend binary (main executable)
│   │   ├── tairseach-mcp       # MCP bridge (external bin)
│   │   └── op-helper           # 1Password helper (external bin)
│   ├── Resources/
│   │   ├── AppIcon.icns        # App icon
│   │   └── assets/             # Frontend assets (JS/CSS/fonts)
│   └── Frameworks/             # System frameworks (none currently)
```

**Binary resolution:**

Tauri resolves external binaries at runtime via:
```rust
tauri::api::process::Command::new_sidecar("tairseach-mcp")
```

Sidecar binaries are located in `Contents/MacOS/` relative to the main executable.

---

## Code Signing

**Status:** Not currently configured (unsigned builds)

**Future setup:**

1. **Apple Developer Certificate:** Required for distribution outside Mac App Store
2. **Signing identity:** Set in `tauri.conf.json`:
   ```json
   "macOS": {
     "signingIdentity": "Developer ID Application: Your Name (TEAMID)"
   }
   ```
3. **Notarization:** Required for macOS 10.15+ to avoid Gatekeeper warnings
4. **Entitlements:** Already configured in `entitlements.plist`:
   - `com.apple.security.files.user-selected.read-write` (file access)
   - `com.apple.security.network.client` (network access)
   - `com.apple.security.cs.allow-unsigned-executable-memory` (may be needed for Rust JIT)

**Build with signing:**
```bash
cargo tauri build -- --sign "Developer ID Application: Your Name"
```

**Notarization:**
```bash
xcrun notarytool submit Tairseach_0.1.0_aarch64.dmg \
  --apple-id your@email.com \
  --password <app-specific-password> \
  --team-id TEAMID
```

---

## Deployment Workflow

### Local Installation

**Quick launch:**
```bash
npm run app:launch
```
Runs `scripts/build-and-launch.sh` (builds and opens app).

**Manual installation:**
```bash
# Build
cargo tauri build

# Copy to Applications
cp -R src-tauri/target/release/bundle/macos/Tairseach.app ~/Applications/

# Launch
open ~/Applications/Tairseach.app
```

### Distribution

**Option 1: Direct .app distribution**
- Zip the `.app` bundle
- Users unzip and drag to Applications folder
- **Requires code signing + notarization** to avoid Gatekeeper blocking

**Option 2: DMG installer**
- Distribute `Tairseach_0.1.0_aarch64.dmg`
- Users mount DMG, drag app to Applications folder
- DMG created automatically by `cargo tauri build`

---

## Runtime Paths

**App binary location:**
```
~/Applications/Tairseach.app/Contents/MacOS/Tairseach
```

**User data directory:**
```
~/.tairseach/
```
(See `docs/reference/environment.md` for full path reference)

**Logs:**
```
~/.tairseach/logs/
```

---

## Build Configuration

### tauri.conf.json

**Key sections:**

**`build.beforeBuildCommand`:**
```json
"npm run build && cargo build -p tairseach-mcp --release"
```
Ensures frontend and external binaries are built before bundling.

**`build.frontendDist`:**
```json
"../dist"
```
Vite output directory (relative to `src-tauri/`).

**`bundle.externalBin`:**
```json
[
  "binaries/tairseach-mcp",
  "binaries/op-helper"
]
```
Binaries copied into `.app` bundle. Paths are relative to `src-tauri/`.

**Note:** You must manually copy built binaries to `src-tauri/binaries/` before running `cargo tauri build`:
```bash
mkdir -p src-tauri/binaries
cp src-tauri/target/release/tairseach-mcp src-tauri/binaries/
cp src-tauri/bin/op-helper src-tauri/binaries/
```

**`bundle.icon`:**
```json
[
  "icons/32x32.png",
  "icons/128x128.png",
  "icons/128x128@2x.png",
  "icons/icon.icns",
  "icons/icon.png"
]
```
Multi-resolution icon set. ICNS is primary for macOS.

---

### package.json Scripts

```json
"scripts": {
  "dev": "vite",
  "build": "vue-tsc --noEmit && vite build",
  "tauri": "tauri",
  "app:build": "tauri build",
  "app:launch": "./scripts/build-and-launch.sh",
  "app:open": "open src-tauri/target/release/bundle/macos/Tairseach.app"
}
```

**`npm run dev`:** Start Vite dev server  
**`npm run build`:** Compile frontend to `dist/`  
**`npm run app:build`:** Full production build (frontend + Rust + bundle)  
**`npm run app:launch`:** Build and launch in one command  
**`npm run app:open`:** Open built app without rebuilding

---

### Cargo.toml

**Key sections:**

**Library crate type:**
```toml
[lib]
crate-type = ["lib", "cdylib", "staticlib"]
```
`cdylib` is required for Tauri to link the Rust backend.

**Dependencies:**
- `tauri = "2"` — Tauri runtime
- `tauri-plugin-shell = "2"` — Shell command execution plugin
- `serde`, `serde_json` — Serialization for Tauri commands
- `tokio` — Async runtime for Rust backend
- `notify` — File system watcher (for manifest hot-reload)
- `aes-gcm`, `rand`, `sha2` — Encryption for credential storage
- `reqwest` — HTTP client for OAuth flows

**Build dependencies:**
```toml
[build-dependencies]
tauri-build = { version = "2", features = [] }
```
Required for Tauri build script.

---

## CI/CD Considerations

**GitHub Actions example:**

```yaml
name: Build Tairseach

on:
  push:
    branches: [main]

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: aarch64-apple-darwin
      
      - name: Setup Go
        uses: actions/setup-go@v4
        with:
          go-version: '1.21'
      
      - name: Install dependencies
        run: npm install
      
      - name: Build external binaries
        run: |
          cargo build -p tairseach-mcp --release
          cd src-tauri/helpers/onepassword && ./build.sh
          mkdir -p src-tauri/binaries
          cp src-tauri/target/release/tairseach-mcp src-tauri/binaries/
          cp src-tauri/bin/op-helper src-tauri/binaries/
      
      - name: Build Tauri app
        run: cargo tauri build
      
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: Tairseach.app
          path: src-tauri/target/release/bundle/macos/Tairseach.app
```

---

## Troubleshooting

**Issue:** `external bin not found: tairseach-mcp`  
**Fix:** Ensure binary is in `src-tauri/binaries/` before build:
```bash
mkdir -p src-tauri/binaries
cp src-tauri/target/release/tairseach-mcp src-tauri/binaries/
```

**Issue:** App crashes on launch  
**Fix:** Check `~/.tairseach/logs/app.log` for Rust panic messages

**Issue:** Unsigned app blocked by Gatekeeper  
**Fix:**
1. Right-click app → Open (allows running unsigned app once)
2. Or: Remove quarantine attribute:
   ```bash
   xattr -dr com.apple.quarantine ~/Applications/Tairseach.app
   ```
3. Or: Code sign + notarize for production

**Issue:** `op-helper` not executing  
**Fix:** Ensure Go binary has execute permissions:
```bash
chmod +x src-tauri/binaries/op-helper
```

**Issue:** Frontend changes not reflected in build  
**Fix:** Clear Vite cache and rebuild:
```bash
rm -rf dist/ node_modules/.vite
npm run build
cargo tauri build
```

---

## Performance Considerations

**Bundle size:**
- Rust binary: ~10MB (debug), ~3MB (release)
- Frontend assets: ~500KB (minified + gzipped)
- External binaries: ~5MB (Go) + ~2MB (Rust MCP)
- **Total app bundle:** ~12-15MB

**Startup time:**
- Cold start: ~500ms (includes MCP server spawn)
- Warm start: ~200ms (macOS app cache)

**Memory footprint:**
- Tauri webview: ~100MB
- Rust backend: ~20MB
- MCP server: ~15MB
- **Total:** ~135MB

---

## Future Enhancements

- [ ] Auto-update via Tauri updater plugin
- [ ] Code signing + notarization automation
- [ ] Windows/Linux cross-platform builds
- [ ] Smaller bundle size (strip unused frontend deps)
- [ ] Pre-built binaries for faster CI/CD
- [ ] Split external binaries into separate installers (reduce app size)
