# Tairseach Build Guide

## Quick Build (Recommended)

Run the complete build script:

```bash
./scripts/build.sh
```

This performs all steps automatically:
1. ✓ Kills any running Tairseach instances
2. ✓ Builds frontend (Vue/Vite) and backend (Rust/Tauri)
3. ✓ Signs with Developer ID certificate
4. ✓ Verifies code signature
5. ✓ Resets TCC permissions cache

After build, run with:
```bash
open ~/environment/tairseach/src-tauri/target/release/bundle/macos/Tairseach.app
```

Or with auto-launch:
```bash
./scripts/build.sh --dev
```

---

## Manual Build Steps

If you need to run steps individually:

### 1. Build Frontend + Backend

```bash
cd ~/environment/tairseach
npm run tauri build
```

**Important:** Always use `npm run tauri build`, not just `cargo build`. The Tauri build embeds the frontend into the binary.

### 2. Sign the App

```bash
codesign --force --deep --options runtime \
    --entitlements src-tauri/entitlements.plist \
    -s "Developer ID Application: ALEXANDER DAVID CONROY (ANRLR4YMQV)" \
    src-tauri/target/release/bundle/macos/Tairseach.app
```

### 3. Verify Signature

```bash
codesign -dv src-tauri/target/release/bundle/macos/Tairseach.app
```

Should show:
- `Identifier=com.naonur.tairseach`
- `TeamIdentifier=ANRLR4YMQV`
- `Signature size=...` (not `adhoc`)

### 4. Reset TCC Permissions

```bash
tccutil reset All com.naonur.tairseach
```

This clears any cached permission denials from previous builds.

---

## Development Mode

For rapid iteration without full rebuild:

```bash
npm run tauri dev
```

**Note:** Dev mode doesn't have proper code signing, so some permission triggers may not work correctly.

---

## Troubleshooting

### App won't open / crashes on launch

1. Check if frontend was included:
   ```bash
   # Should show embedded resources
   ls -la src-tauri/target/release/bundle/macos/Tairseach.app/Contents/
   ```
   
2. Run from terminal to see errors:
   ```bash
   src-tauri/target/release/bundle/macos/Tairseach.app/Contents/MacOS/tairseach
   ```

### Permissions not working

1. Verify entitlements are embedded:
   ```bash
   codesign -dv --entitlements - src-tauri/target/release/bundle/macos/Tairseach.app
   ```
   
2. Reset TCC:
   ```bash
   tccutil reset All com.naonur.tairseach
   ```

### Code signing fails

1. Check certificate is installed:
   ```bash
   security find-identity -v -p codesigning
   ```
   
2. Should show:
   ```
   "Developer ID Application: ALEXANDER DAVID CONROY (ANRLR4YMQV)"
   ```

### DMG creation fails

This is a known issue and doesn't affect the app. The `.app` bundle is created successfully.

---

## Build Artifacts

| Path | Description |
|------|-------------|
| `src-tauri/target/release/bundle/macos/Tairseach.app` | Signed app bundle |
| `src-tauri/target/release/tairseach` | Raw binary (unsigned) |
| `dist/` | Frontend build output |

---

## CI/CD Notes

For future automation:

```yaml
# Example GitHub Actions step
- name: Build Tairseach
  run: |
    npm ci
    npm run tauri build
    
- name: Sign (macOS only)
  if: runner.os == 'macOS'
  run: |
    codesign --force --deep --options runtime \
      --entitlements src-tauri/entitlements.plist \
      -s "${{ secrets.APPLE_SIGNING_IDENTITY }}" \
      src-tauri/target/release/bundle/macos/Tairseach.app
```

---

*🪶 Build with care. The threshold must be stable.*
