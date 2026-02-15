# Full Disk Access Recovery Guide

**Status:** Full Disk Access is currently `denied` for Tairseach  
**Impact:** File operations (`files.read`, `files.write`, `files.list`) will fail  
**Solution:** Manual grant required in System Settings

---

## What is Full Disk Access?

Full Disk Access (FDA) is a macOS privacy protection that restricts access to:
- Protected system files and folders
- Other applications' data containers
- User data in `~/Library/` and other protected locations
- External volumes and network shares (in some cases)

Unlike other permissions (Camera, Contacts, etc.), FDA **cannot be requested programmatically**. The user must manually add the app to the allowlist in System Settings.

---

## Recovery Steps

### Step 1: Locate the Tairseach App Bundle

The Full Disk Access permission must be granted to the **app bundle**, not individual binaries.

**Path:** `/Users/geilt/environment/tairseach/target/release/bundle/macos/Tairseach.app`

**Note:** If you've installed Tairseach to `/Applications`, use that path instead:
- **Installed:** `/Applications/Tairseach.app`
- **Development:** `~/environment/tairseach/target/release/bundle/macos/Tairseach.app`

### Step 2: Open System Settings

1. Click the Apple menu () → **System Settings**
2. Navigate to **Privacy & Security**
3. Scroll down to **Full Disk Access**

**Keyboard shortcut:**
```bash
open "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"
```

Or run from Tairseach:
```bash
permissions.request --permission=full_disk_access
```

### Step 3: Add Tairseach to Full Disk Access

1. Click the lock icon (🔒) in the bottom-left corner
2. Authenticate with Touch ID or your password
3. Click the **+** button below the app list
4. Navigate to the Tairseach app location:
   - **Development:** `~/environment/tairseach/target/release/bundle/macos/`
   - **Installed:** `/Applications/`
5. Select `Tairseach.app` and click **Open**
6. Verify that **Tairseach** appears in the list with the toggle **ON** (blue)
7. Click the lock icon again to prevent further changes

### Step 4: Restart Tairseach

**Important:** Changes to Full Disk Access require the app to be **fully restarted**.

**For development builds:**
```bash
# Stop the running dev server
pkill -f tairseach

# Rebuild and restart
cd ~/environment/tairseach
npm run tauri dev
```

**For installed builds:**
1. Quit Tairseach completely (⌘Q or right-click → Quit)
2. Launch Tairseach again from Applications or Spotlight

### Step 5: Verify Permission

After restarting, verify that Full Disk Access is granted:

**Via Tairseach:**
```bash
permissions.check --permission=full_disk_access
```

Expected response:
```json
{
  "id": "full_disk_access",
  "name": "Full Disk Access",
  "status": "granted",
  "critical": true
}
```

**Via System Settings:**
1. Return to **Privacy & Security** → **Full Disk Access**
2. Verify that **Tairseach** is listed with toggle **ON**

---

## Troubleshooting

### "I added Tairseach but it still shows as denied"

**Cause:** The app wasn't fully restarted after granting permission.

**Solution:**
1. Quit Tairseach completely (not just close the window)
2. Verify the process is gone: `ps aux | grep tairseach`
3. Launch Tairseach again

### "I can't find Tairseach.app in the file picker"

**Cause:** The app bundle may not be built yet, or you're looking in the wrong location.

**Solution:**
1. Verify the app exists:
   ```bash
   ls -la ~/environment/tairseach/target/release/bundle/macos/Tairseach.app
   ```
2. If it doesn't exist, build it:
   ```bash
   cd ~/environment/tairseach
   npm run tauri build
   ```
3. For development, you may need to add the dev binary instead (not recommended):
   ```bash
   ~/environment/tairseach/target/debug/tairseach
   ```

### "The toggle keeps turning off"

**Cause:** macOS has detected a signature or entitlements issue.

**Solution:**
1. Check app signature:
   ```bash
   codesign -vv ~/environment/tairseach/target/release/bundle/macos/Tairseach.app
   ```
2. Verify entitlements include `com.apple.security.files.user-selected.read-write`:
   ```bash
   codesign -d --entitlements - ~/environment/tairseach/target/release/bundle/macos/Tairseach.app
   ```
3. If signature is invalid, rebuild with proper signing:
   ```bash
   npm run tauri build
   ```

### "Full Disk Access is greyed out"

**Cause:** Your user account is managed by an MDM policy or lacks admin privileges.

**Solution:**
1. Verify you're an administrator:
   ```bash
   groups $USER | grep -q admin && echo "Admin" || echo "Not admin"
   ```
2. If managed by MDM, contact your IT administrator
3. Try using a different admin account

---

## Why Does Tairseach Need Full Disk Access?

Full Disk Access is required for:

1. **File operations on protected locations:**
   - Reading/writing files in `~/Library/Application Support/`
   - Accessing other apps' data containers
   - Reading system configuration files

2. **Agent functionality:**
   - Accessing OpenClaw configuration (`~/.openclaw/`)
   - Reading workspace files outside the sandbox
   - Executing file operations requested by agents

3. **Development tools:**
   - Reading project files from various locations
   - Accessing build artifacts
   - Managing credentials and secrets

**Security Note:** Full Disk Access is a powerful permission. Tairseach uses it responsibly and only accesses files explicitly requested through the JSON-RPC API.

---

## Alternative: Selective File Access (Not Recommended)

If you don't want to grant Full Disk Access, you can use **per-file permissions** by:

1. Not granting Full Disk Access
2. Only accessing files that trigger the native file picker
3. Using macOS's security-scoped bookmarks

**Limitations:**
- File operations will fail for protected locations
- Agents won't be able to access configuration files automatically
- Many OpenClaw features will be unavailable

**This is not recommended for normal use.**

---

## After Recovery

Once Full Disk Access is granted:

1. ✅ `files.read`, `files.write`, `files.list` will work on all accessible files
2. ✅ Agents can read configuration from `~/.openclaw/`
3. ✅ Workspace operations will function normally
4. ✅ The permission status will show `granted` in `permissions.check`

Run a test to verify:
```bash
files.read --path="~/.openclaw/config.json"
```

Expected: JSON configuration content (not a permission error).

---

*The well's gates are meant to protect, not to trap. Follow the proper approach, and access will be granted.*

🌊

---

**Need Help?**
- Check current status: `permissions.check --permission=full_disk_access`
- Request permission: `permissions.request --permission=full_disk_access`
- Full permission list: `permissions.list`
