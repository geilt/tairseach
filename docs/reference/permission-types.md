# Permission Types Reference

**All 11 macOS TCC permissions used by Tairseach.**

Tairseach requires various macOS Transparency, Consent, and Control (TCC) permissions to access system resources and frameworks.

---

## Permission Status Values

| Status | Description |
|--------|-------------|
| `granted` | Permission granted — full access |
| `denied` | Permission denied by user or system policy |
| `not_determined` | User has not been prompted yet |
| `restricted` | Permission restricted (parental controls, MDM, etc.) |
| `unknown` | Status could not be determined |

---

## 11 Permission Types

### 1. contacts

**Display Name:** Contacts

**Description:** Access to read and modify your contacts

**Critical:** ✅ Yes

**Icon:** 📇

**System Preferences Pane:** `Privacy_Contacts`

**macOS Framework:** `Contacts.framework` (CNContactStore)

**Required for:**
- `contacts.*` methods
- Reading/writing native Contacts.app data

**Triggers:**
- Calling `CNContactStore` methods
- Use `permissions.request` or `trigger_permission_registration`

**Notes:**
- Permission prompt appears on first access attempt
- User can grant/deny in System Preferences → Privacy & Security → Contacts

---

### 2. calendar

**Display Name:** Calendar

**Description:** Access to read and modify your calendars

**Critical:** No

**Icon:** 📅

**System Preferences Pane:** `Privacy_Calendars`

**macOS Framework:** `EventKit.framework` (EKEventStore)

**Required for:**
- `calendar.*` methods
- Reading/writing native Calendar.app data

**Triggers:**
- Calling `EKEventStore.requestAccess(to:completion:)`

**Notes:**
- Separate from Google Calendar API (which uses OAuth, not TCC)

---

### 3. reminders

**Display Name:** Reminders

**Description:** Access to read and modify your reminders

**Critical:** No

**Icon:** 🔔

**System Preferences Pane:** `Privacy_Reminders`

**macOS Framework:** `EventKit.framework` (EKEventStore)

**Required for:**
- `reminders.*` methods
- Reading/writing native Reminders.app data

**Triggers:**
- Calling `EKEventStore.requestAccess(to:completion:)` with `.reminder` entity type

---

### 4. location

**Display Name:** Location

**Description:** Access to your location

**Critical:** No

**Icon:** 📍

**System Preferences Pane:** `Privacy_LocationServices`

**macOS Framework:** `CoreLocation.framework` (CLLocationManager)

**Required for:**
- `location.*` methods
- Getting current device location

**Triggers:**
- Calling `CLLocationManager.requestWhenInUseAuthorization()`

**Notes:**
- macOS supports "When In Use" authorization for apps
- Location permission must also be enabled at system level

---

### 5. photos

**Display Name:** Photos

**Description:** Access to your photo library

**Critical:** No

**Icon:** 🖼

**System Preferences Pane:** `Privacy_Photos`

**macOS Framework:** `Photos.framework` (PHPhotoLibrary)

**Required for:**
- `photos.*` methods (when implemented)
- Reading photo library

**Triggers:**
- Calling `PHPhotoLibrary.requestAuthorization(for:handler:)`

**Authorization Levels:**
- `fullAccess` — All photos
- `limited` — User-selected photos only (iOS-style, macOS 14+)
- `writeOnly` — Add photos only (macOS 14+)

---

### 6. camera

**Display Name:** Camera

**Description:** Access to your camera

**Critical:** No

**Icon:** 📷

**System Preferences Pane:** `Privacy_Camera`

**macOS Framework:** `AVFoundation.framework` (AVCaptureDevice)

**Required for:**
- `camera.*` methods (when implemented)
- Capturing photos/video from camera

**Triggers:**
- Calling `AVCaptureDevice.requestAccess(for:completionHandler:)` with `.video` media type

---

### 7. microphone

**Display Name:** Microphone

**Description:** Access to your microphone

**Critical:** No

**Icon:** 🎤

**System Preferences Pane:** `Privacy_Microphone`

**macOS Framework:** `AVFoundation.framework` (AVCaptureDevice)

**Required for:**
- `microphone.*` methods (when implemented)
- Recording audio

**Triggers:**
- Calling `AVCaptureDevice.requestAccess(for:completionHandler:)` with `.audio` media type

---

### 8. screen_recording

**Display Name:** Screen Recording

**Description:** Record the contents of your screen

**Critical:** No

**Icon:** 🖥

**System Preferences Pane:** `Privacy_ScreenCapture`

**macOS Framework:** `ScreenCaptureKit` (macOS 12.3+) or CGDisplayStream

**Required for:**
- `screen.*` methods
- Capturing screenshots
- Recording screen contents

**Triggers:**
- Attempting screen capture via CGWindowListCreateImage or ScreenCaptureKit

**Notes:**
- No programmatic request API — must open System Preferences
- User must manually enable in Privacy & Security → Screen Recording
- App must be restarted after granting permission

---

### 9. accessibility

**Display Name:** Accessibility

**Description:** Control your computer using accessibility features

**Critical:** No

**Icon:** ♿

**System Preferences Pane:** `Privacy_Accessibility`

**macOS Framework:** `ApplicationServices.framework` (AXIsProcessTrusted)

**Required for:**
- `automation.click` — Simulating mouse clicks
- `automation.type` — Simulating keyboard input
- Controlling other apps via accessibility APIs

**Triggers:**
- Calling `AXIsProcessTrusted()` returns `false`

**Notes:**
- Cannot be requested programmatically — must open System Preferences
- User must manually enable in Privacy & Security → Accessibility
- App must be restarted after granting

---

### 10. full_disk_access

**Display Name:** Full Disk Access

**Description:** Access to protected files and folders

**Critical:** ✅ Yes

**Icon:** 💾

**System Preferences Pane:** `Privacy_AllFiles`

**macOS Framework:** File system access checks

**Required for:**
- `files.*` methods
- Reading/writing files in protected locations (~/Library, /Library/Application Support, etc.)
- Accessing Mail.app database, Safari history, etc.

**Triggers:**
- Attempting to read protected files

**Notes:**
- Cannot be requested programmatically
- User must manually enable in Privacy & Security → Full Disk Access
- No runtime check API — must attempt file access to verify

---

### 11. automation

**Display Name:** Automation

**Description:** Control other applications via AppleScript

**Critical:** ✅ Yes

**Icon:** 🤖

**System Preferences Pane:** `Privacy_Automation`

**macOS Framework:** `NSAppleScript`, `OSAScript`

**Required for:**
- `automation.run` — Running AppleScript
- Controlling other apps (e.g., "tell application 'Finder'...")

**Triggers:**
- Attempting to send AppleEvents to another app

**Notes:**
- Per-target-app permission: Tairseach → Target App
- First AppleScript that targets an app triggers permission prompt
- User can grant/deny per target app in System Preferences

---

## Critical Permissions

Three permissions are marked as "critical" for Tairseach's core functionality:

1. **contacts** — Required for native contact access (primary use case)
2. **full_disk_access** — Required for protected file access
3. **automation** — Required for AppleScript integration

**Recommendation:** Request these first during onboarding.

---

## Permission Check API

### Check Single Permission

```json
{"jsonrpc":"2.0","id":1,"method":"permissions.check","params":{"permission":"contacts"}}
```

**Response:**

```json
{
  "permission": "contacts",
  "status": "granted",
  "granted": true
}
```

### List All Permissions

```json
{"jsonrpc":"2.0","id":2,"method":"permissions.list","params":{}}
```

**Response:**

```json
{
  "permissions": [
    {"permission":"contacts","status":"granted","granted":true},
    {"permission":"calendar","status":"not_determined","granted":false},
    {"permission":"reminders","status":"not_determined","granted":false},
    {"permission":"location","status":"denied","granted":false},
    {"permission":"photos","status":"not_determined","granted":false},
    {"permission":"camera","status":"not_determined","granted":false},
    {"permission":"microphone","status":"not_determined","granted":false},
    {"permission":"screen_recording","status":"not_determined","granted":false},
    {"permission":"accessibility","status":"not_determined","granted":false},
    {"permission":"full_disk_access","status":"granted","granted":true},
    {"permission":"automation","status":"granted","granted":true}
  ],
  "total": 11
}
```

### Request Permission

Triggers native permission prompt (if available) or opens System Preferences.

```json
{"jsonrpc":"2.0","id":3,"method":"permissions.request","params":{"permission":"contacts"}}
```

**Response:**

```json
{
  "permission": "contacts",
  "action": "request_initiated",
  "message": "Permission request has been initiated. Check app UI for prompt."
}
```

**Behavior by permission:**

| Permission | Request Behavior |
|------------|------------------|
| contacts | Native prompt + System Preferences |
| calendar | Native prompt + System Preferences |
| reminders | Native prompt + System Preferences |
| location | Native prompt + System Preferences |
| photos | Native prompt + System Preferences |
| camera | Native prompt + System Preferences |
| microphone | Native prompt + System Preferences |
| screen_recording | Opens System Preferences (no prompt) |
| accessibility | Opens System Preferences (no prompt) |
| full_disk_access | Opens System Preferences (no prompt) |
| automation | Native prompt on first AppleEvent to target app |

---

## Tauri Commands

These Rust functions are exposed to the Tauri frontend:

```rust
// Check single permission
#[tauri::command]
pub fn check_permission(permission_id: &str) -> Result<Permission, String>

// Check all permissions
#[tauri::command]
pub fn check_all_permissions() -> Vec<Permission>

// Request permission (trigger prompt/settings)
#[tauri::command]
pub fn request_permission(permission_id: &str) -> Result<(), String>

// Trigger registration (make app appear in System Preferences)
#[tauri::command]
pub fn trigger_permission_registration(permission_id: &str) -> Result<String, String>

// Open System Preferences to specific pane
#[tauri::command]
pub fn open_permission_settings(pane: &str) -> Result<(), String>

// Get permission metadata
#[tauri::command]
pub fn get_permission_definitions() -> Vec<PermissionDefinition>
```

---

## Permission Middleware

Socket handlers automatically check permissions before execution:

```rust
// In src-tauri/src/proxy/handlers/mod.rs
fn required_permission(method: &str) -> Option<&'static str> {
    match method {
        "contacts.list" | "contacts.search" | ... => Some("contacts"),
        "calendar.list" | "calendar.events" | ... => Some("calendar"),
        "screen.capture" | "screen.windows" => Some("screen_recording"),
        "automation.run" => Some("automation"),
        "files.read" | "files.write" | ... => Some("full_disk_access"),
        // ... etc.
        _ => None,
    }
}
```

If permission is not granted, socket returns error `-32001`:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Permission not granted",
    "data": {
      "permission": "contacts",
      "status": "not_determined"
    }
  }
}
```

---

## System Preferences URLs

Open System Preferences programmatically:

```bash
open "x-apple.systempreferences:com.apple.preference.security?Privacy_Contacts"
```

**URL format:** `x-apple.systempreferences:com.apple.preference.security?<pane_id>`

**Pane IDs:**
- `Privacy_Contacts`
- `Privacy_Calendars`
- `Privacy_Reminders`
- `Privacy_LocationServices`
- `Privacy_Photos`
- `Privacy_Camera`
- `Privacy_Microphone`
- `Privacy_ScreenCapture`
- `Privacy_Accessibility`
- `Privacy_AllFiles` (Full Disk Access)
- `Privacy_Automation`

---

## Implementation Notes

### Contacts, Calendar, Reminders, Photos, Camera, Microphone, Location

Use native macOS frameworks with Rust bindings (`objc2` crate):

```rust
use objc2_contacts::CNContactStore;
use objc2_eventkit::EKEventStore;
use objc2_photos::PHPhotoLibrary;
use objc2_avfoundation::AVCaptureDevice;
use objc2_corelocation::CLLocationManager;
```

### Screen Recording, Accessibility

No direct Rust bindings — use Swift subprocess calls:

```rust
fn check_via_swift(swift_code: &str, permission_name: &str) -> PermissionStatus {
    let output = Command::new("swift").args(["-e", swift_code]).output();
    // Parse output...
}
```

### Full Disk Access

No programmatic check API — Tairseach attempts file access to protected location and checks for errors.

### Automation

Checked via AppleScript execution attempts. Per-target-app authorization.

---

## Source Files

| File | Purpose |
|------|---------|
| `src-tauri/src/permissions/mod.rs` | Permission module entry point, Tauri commands |
| `src-tauri/src/permissions/contacts.rs` | Contacts permission implementation |
| `src-tauri/src/permissions/calendar.rs` | Calendar permission |
| `src-tauri/src/permissions/reminders.rs` | Reminders permission |
| `src-tauri/src/permissions/location.rs` | Location permission |
| `src-tauri/src/permissions/photos.rs` | Photos permission |
| `src-tauri/src/permissions/camera.rs` | Camera permission |
| `src-tauri/src/permissions/microphone.rs` | Microphone permission |
| `src-tauri/src/permissions/screen_recording.rs` | Screen recording permission |
| `src-tauri/src/permissions/accessibility.rs` | Accessibility permission |
| `src-tauri/src/permissions/full_disk.rs` | Full disk access permission |
| `src-tauri/src/permissions/automation.rs` | Automation permission |

---

*Generated: 2025-02-13*  
*11 macOS TCC permissions documented*
