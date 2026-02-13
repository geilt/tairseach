# Deployment Architecture

**Platform:** macOS (darwin/arm64)  
**Build system:** Tauri 2 + Cargo + Vite  
**Bundling:** macOS .app + .dmg  
**Target location:** `~/Applications/Tairseach.app`

---

## Build Overview

Tairseach is a Tauri 2 application consisting of:

1. **Vue 3 frontend** — Built by Vite, bundled into `dist/`
2. **Rust backend** — Compiled by Cargo as a native macOS binary
3. **External binaries** — `tairseach-mcp` (Rust) + `op-helper` (Go)
4. **Code signing** — Entitlements for system access (no sandbox)

---

## Build Commands

### Development

```bash
npm run dev          # Start Vite dev server + Tauri dev mode
npm run tauri dev    # Same as above
```

**Dev mode:**
- Frontend at `http://localhost:1420`
- Hot reload for Vue components
- Rust backend recompiles on change
- Does NOT register as system app (permissions won't work properly)

### Production

```bash
npm run app:build    # Full build: Vite + Cargo + bundle

# Equivalent to:
npm run build                           # Vue build (vite build)
cargo tauri build                      # Tauri bundle
```

**Output:**
```
src-tauri/target/release/bundle/macos/
├── Tairseach.app/              # macOS application bundle
└── Tairseach_0.1.0_aarch64.dmg # DMG installer
```

### Launch Scripts

```bash
npm run app:launch   # Build + patch + open (scripts/build-and-launch.sh)
npm run app:open     # Open existing build without rebuilding
```

**`scripts/build-and-launch.sh`:**
1. Runs `npx tauri build`
2. Patches `Info.plist` with privacy descriptions (via `scripts/patch-info-plist.sh`)
3. Opens `Tairseach.app`

---

## Build Configuration

### `tauri.conf.json`

**Key settings:**

```json
{
  "productName": "Tairseach",
  "version": "0.1.0",
  "identifier": "com.naonur.tairseach",
  
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build && cargo build -p tairseach-mcp --release",
    "frontendDist": "../dist"
  },
  
  "app": {
    "windows": [{
      "title": "Tairseach",
      "width": 1200,
      "height": 800,
      "minWidth": 900,
      "minHeight": 600
    }]
  },
  
  "bundle": {
    "active": true,
    "targets": "all",
    "externalBin": [
      "binaries/tairseach-mcp",
      "binaries/op-helper"
    ],
    "macOS": {
      "entitlements": "entitlements.plist",
      "dmg": {
        "appPosition": { "x": 180, "y": 170 },
        "applicationFolderPosition": { "x": 480, "y": 170 },
        "windowSize": { "width": 660, "height": 400 }
      }
    }
  }
}
```

**Critical fields:**

- **`beforeBuildCommand`** — Compiles `tairseach-mcp` binary before bundling
- **`externalBin`** — Copies pre-built binaries into `.app/Contents/MacOS/`
- **`entitlements`** — Grants system permissions (see below)

### Binary Resolution

Tauri expects arch-specific binaries in `src-tauri/binaries/`:

```
src-tauri/binaries/
├── tairseach-mcp-aarch64-apple-darwin   # MCP server (Rust)
└── op-helper-aarch64-apple-darwin       # 1Password helper (Go)
```

During bundling, Tauri strips the arch suffix and copies them as:

```
Tairseach.app/Contents/MacOS/
├── tairseach-mcp
└── op-helper
```

---

## External Binaries

### 1. `tairseach-mcp` (Rust)

**Purpose:** MCP-to-macOS API bridge server (contacts, calendar, automation, etc.)

**Build:**
```bash
cd src-tauri
cargo build -p tairseach-mcp --release
cp target/release/tairseach-mcp binaries/tairseach-mcp-aarch64-apple-darwin
```

**Triggered by:** `beforeBuildCommand` in `tauri.conf.json`

**Dependencies:** See `src-tauri/Cargo.toml` (below)

### 2. `op-helper` (Go)

**Purpose:** 1Password SDK integration (Go SDK → C FFI → Rust)

**Build:**
```bash
cd src-tauri/helpers/onepassword
./build.sh
```

**`build.sh` steps:**
1. Initialize Go module if missing (`go mod init tairseach-op-helper`)
2. Fetch 1Password SDK: `go get github.com/1password/onepassword-sdk-go@latest`
3. Compile for `darwin/arm64`: `GOOS=darwin GOARCH=arm64 go build -o ../../bin/op-helper main.go`
4. Copy to `src-tauri/binaries/op-helper-aarch64-apple-darwin`

**Manual build required:** Not automated in `beforeBuildCommand` (run once after SDK changes)

**Source:** `src-tauri/helpers/onepassword/main.go`

---

## Rust Dependencies

### `Cargo.toml`

**Core Tauri:**
```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**Async runtime:**
```toml
tokio = { version = "1", features = ["full", "net", "io-util", "sync", "macros", "rt-multi-thread"] }
```

**File watching:**
```toml
notify = { version = "6", default-features = false, features = ["macos_fsevent"] }
```

**Auth broker (encryption):**
```toml
aes-gcm = "0.10"
sha2 = "0.10"
hkdf = "0.12"
rand = "0.8"
base64 = "0.22"
hex = "0.4"
zeroize = { version = "1", features = ["derive"] }  # Secure memory zeroing
```

**HTTP client:**
```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```

**macOS system APIs:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6"
objc2-foundation = { version = "0.3", features = ["NSString", "NSObject", ...] }
objc2-contacts = { version = "0.3", features = ["CNContactStore", "CNContact", ...] }
objc2-event-kit = { version = "0.3", features = ["EKEventStore", "EKEvent", "EKReminder"] }
objc2-photos = { version = "0.3", features = ["PHPhotoLibrary", "PHAsset"] }
objc2-av-foundation = { version = "0.3", features = ["AVCaptureDevice"] }
objc2-core-location = { version = "0.3", features = ["CLLocationManager", ...] }
block2 = "0.6"
```

**Optional (v1 migration only):**
```toml
[target.'cfg(target_os = "macos")'.dependencies.security-framework]
version = "2.11"
optional = true
```

### Build Profile

**Release optimizations:**
```toml
[profile.release]
panic = "abort"        # Smaller binary
codegen-units = 1      # Better optimization
lto = true             # Link-time optimization
opt-level = "s"        # Optimize for size
strip = true           # Remove debug symbols
```

---

## Code Signing & Entitlements

### `Entitlements.plist`

Tairseach requires **NO sandbox** to access system APIs.

**Permissions granted:**

```xml
<!-- No sandbox - we need full system access -->
<key>com.apple.security.app-sandbox</key>
<false/>

<!-- Automation / Apple Events -->
<key>com.apple.security.automation.apple-events</key>
<true/>

<!-- Personal data -->
<key>com.apple.security.personal-information.addressbook</key>
<true/>
<key>com.apple.security.personal-information.calendars</key>
<true/>
<key>com.apple.security.personal-information.reminders</key>
<true/>
<key>com.apple.security.personal-information.photos-library</key>
<true/>
<key>com.apple.security.personal-information.location</key>
<true/>

<!-- Hardware -->
<key>com.apple.security.device.camera</key>
<true/>
<key>com.apple.security.device.audio-input</key>
<true/>

<!-- File access -->
<key>com.apple.security.files.user-selected.read-write</key>
<true/>
```

**Why no sandbox?**
- System automation requires full Apple Event access
- Contacts/Calendar frameworks need direct API calls
- Unix socket communication (`~/.tairseach/tairseach.sock`)
- Exec command approval system

### Code Signing

**Current state:** Self-signed / ad-hoc signing (for local testing)

**For distribution:**
1. Obtain Apple Developer ID certificate
2. Add to `tauri.conf.json`:
   ```json
   "macOS": {
     "signingIdentity": "Developer ID Application: Your Name (TEAMID)"
   }
   ```
3. Notarize the app:
   ```bash
   xcrun notarytool submit Tairseach.dmg --keychain-profile "AC_PASSWORD" --wait
   xcrun stapler staple Tairseach.app
   ```

---

## Installation

### Development Install

```bash
# Build and launch from source
npm run app:launch

# App installed to:
src-tauri/target/release/bundle/macos/Tairseach.app

# Copy to Applications:
cp -r src-tauri/target/release/bundle/macos/Tairseach.app ~/Applications/
```

### DMG Distribution

```bash
# Build creates DMG at:
src-tauri/target/release/bundle/dmg/Tairseach_0.1.0_aarch64.dmg

# User workflow:
1. Open DMG
2. Drag Tairseach.app to Applications folder
3. Launch from Applications
4. Grant permissions in System Settings
```

### First Launch

**Required permissions:**
1. Automation (System Settings → Privacy & Security → Automation)
2. Contacts (System Settings → Privacy & Security → Contacts)
3. Calendars (System Settings → Privacy & Security → Calendars)
4. Full Disk Access (System Settings → Privacy & Security → Full Disk Access)
5. Accessibility (System Settings → Privacy & Security → Accessibility)

**Runtime setup:**
- Creates `~/.tairseach/` directory
- Generates master key for auth broker
- Starts MCP proxy socket server

---

## Node Dependencies

### `package.json`

**Core dependencies:**
```json
{
  "@tauri-apps/api": "^2.2.0",
  "@tauri-apps/plugin-shell": "^2.2.0",
  "pinia": "^2.3.1",
  "vue": "^3.5.13",
  "vue-router": "^4.5.0"
}
```

**Build tools:**
```json
{
  "@tauri-apps/cli": "^2.2.7",
  "@vitejs/plugin-vue": "^5.2.1",
  "autoprefixer": "^10.4.20",
  "postcss": "^8.5.1",
  "tailwindcss": "^3.4.17",
  "typescript": "~5.7.3",
  "vite": "^6.0.11",
  "vue-tsc": "^2.2.0"
}
```

---

## Build Artifacts

### Directory Structure

```
src-tauri/target/release/
├── tairseach                 # Main binary (not used directly)
├── tairseach-mcp             # MCP server binary
└── bundle/
    ├── macos/
    │   └── Tairseach.app/
    │       ├── Contents/
    │       │   ├── MacOS/
    │       │   │   ├── Tairseach         # Main executable
    │       │   │   ├── tairseach-mcp     # MCP server
    │       │   │   └── op-helper         # 1Password helper
    │       │   ├── Resources/
    │       │   │   ├── icon.icns
    │       │   │   └── dist/ (Vue build)
    │       │   ├── Info.plist
    │       │   └── _CodeSignature/
    │       └── Entitlements.plist
    └── dmg/
        └── Tairseach_0.1.0_aarch64.dmg
```

### Binary Sizes (Typical)

- `Tairseach` (main): ~8 MB (with Rust backend + Tauri runtime)
- `tairseach-mcp`: ~600 KB (stripped release build)
- `op-helper`: ~25 MB (Go runtime + 1Password SDK)
- Total `.app` size: ~35-40 MB

---

## Environment Variables

**Build-time:**
- `CARGO_BUILD_TARGET` — Auto-set to `aarch64-apple-darwin`
- `RUSTFLAGS` — Auto-configured by Cargo profile

**Runtime:**
- `TAIRSEACH_MCP_SOCKET` — Override socket path (default: `~/.tairseach/tairseach.sock`)
- `TAIRSEACH_LOG` — Logging level (trace/debug/info/warn/error)

---

## Dev vs Release Differences

| Aspect | Dev Mode | Release Build |
|--------|----------|---------------|
| Frontend | Hot reload at localhost:1420 | Bundled in `.app/Contents/Resources/dist/` |
| Rust | Unoptimized debug build | LTO + stripped + optimized for size |
| Binaries | Not bundled (must be in PATH) | Embedded in `.app/Contents/MacOS/` |
| Permissions | Not registered with system | Requires user grant via System Settings |
| Code signing | None | Ad-hoc or Developer ID |
| Launch | Terminal (npm run dev) | Double-click app |

---

## CI/CD Notes (Future)

**Recommended setup:**
1. GitHub Actions on macOS runner (arm64)
2. Cache Cargo dependencies
3. Build both `tairseach` and `tairseach-mcp`
4. Build `op-helper` (requires Go toolchain)
5. Bundle with `cargo tauri build`
6. Notarize DMG (requires Apple Developer account)
7. Upload artifacts (DMG + checksums)

**Example workflow:**
```yaml
- name: Build
  run: |
    npm install
    npm run build
    cargo build -p tairseach-mcp --release
    cd src-tauri/helpers/onepassword && ./build.sh
    cargo tauri build --ci
```

---

## Troubleshooting

**"Binary not found" error:**
- Ensure `tairseach-mcp-aarch64-apple-darwin` exists in `src-tauri/binaries/`
- Check `beforeBuildCommand` ran successfully

**Permissions not working in dev mode:**
- Use `npm run app:launch` to test (dev mode doesn't register with system)
- Check `Entitlements.plist` is applied

**Go helper build fails:**
- Install Go 1.20+ (`brew install go`)
- Delete `src-tauri/helpers/onepassword/go.mod` and re-run `build.sh`

**Tauri build fails with linker error:**
- Update Xcode CLI tools: `xcode-select --install`
- Clean build: `cargo clean && npm run app:build`

---

**Source files analyzed:**
- `tauri.conf.json` — Tauri configuration
- `Cargo.toml` — Rust dependencies and build profile
- `package.json` — Node dependencies and scripts
- `Entitlements.plist` — macOS permissions
- `helpers/onepassword/build.sh` — Go helper build script
- `scripts/build-and-launch.sh` — Development launch script

**Last updated:** 2026-02-13
