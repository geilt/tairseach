# Permissions System Architecture

**Component:** macOS TCC Permission Management  
**Location:** `src-tauri/src/permissions/`  
**Platform:** macOS 12.0+ (Monterey or later)  
**Technology:** objc2 bindings to native frameworks

---

## Purpose

The permissions system provides a unified Rust interface to macOS Transparency, Consent, and Control (TCC) permissions. It allows Tairseach to:

1. **Check permission status** — Is permission granted, denied, or not determined?
2. **Request permissions** — Trigger macOS permission prompt
3. **Observe permission changes** — React to user granting/revoking permissions
4. **Enforce permission gates** — Block operations that lack required permissions

## Supported Permissions

| Permission | TCC Identifier | Native Framework | Purpose |
|------------|---------------|------------------|---------|
| **Contacts** | `kTCCServiceAddressBook` | Contacts.framework | Access Contacts.app data |
| **Calendar** | `kTCCServiceCalendar` | EventKit.framework | Access Calendar.app events |
| **Reminders** | `kTCCServiceReminders` | EventKit.framework | Access Reminders.app tasks |
| **Photos** | `kTCCServicePhotos` | Photos.framework | Access Photos library |
| **Camera** | `kTCCServiceCamera` | AVFoundation | Use camera |
| **Microphone** | `kTCCServiceMicrophone` | AVFoundation | Use microphone |
| **Location** | CoreLocation | CoreLocation | Get device location |
| **Screen Recording** | `kTCCServiceScreenCapture` | ScreenCaptureKit | Capture screen |
| **Accessibility** | AXIsProcessTrusted | Accessibility.framework | Control UI elements |
| **Full Disk Access** | File system test | N/A | Read protected files |
| **Automation** | `kTCCServiceAppleEvents` | AppleScript/JXA | Send AppleEvents |

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    PermissionsModule                          │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Public API (Tauri commands)                           │  │
│  │  • get_permissions() → Vec<Permission>                 │  │
│  │  • check_permission(id) → Permission                   │  │
│  │  • request_permission(id)                              │  │
│  │  • open_permission_settings(id)                        │  │
│  └────────────────────┬───────────────────────────────────┘  │
│                       │                                       │
│  ┌────────────────────▼───────────────────────────────────┐  │
│  │  Permission Definitions (ids module)                    │  │
│  │  • CONTACTS, CALENDAR, REMINDERS, ...                  │  │
│  │  • Metadata (name, description, settings_url)          │  │
│  └────────────────────┬───────────────────────────────────┘  │
│                       │                                       │
│           ┌───────────┴───────────┐                          │
│           │                       │                          │
│  ┌────────▼────────┐   ┌─────────▼──────────┐              │
│  │ Permission      │   │ Permission Checker │              │
│  │ Modules         │   │ (per-permission)   │              │
│  │ (11 modules)    │   │                    │              │
│  │ • contacts.rs   │   │ Native Framework   │              │
│  │ • calendar.rs   │   │ FFI Calls:         │              │
│  │ • location.rs   │   │ • CNContactStore   │              │
│  │ • ...           │   │ • EKEventStore     │              │
│  └─────────────────┘   │ • CLLocationMgr    │              │
│                        │ • AVCaptureDevice  │              │
│                        └────────────────────┘              │
└──────────────────────────────────────────────────────────────┘
                         │
        ┌────────────────┴────────────────┐
        │                                 │
┌───────▼───────┐                ┌────────▼────────┐
│ macOS TCC     │                │ System Settings │
│ Database      │                │ (when prompted) │
│ /Library/...  │                └─────────────────┘
│ ~/Library/... │
└───────────────┘
```

## Permission Status Values

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionStatus {
    Granted,        // User approved, app can access resource
    Denied,         // User explicitly denied access
    NotDetermined,  // User hasn't been asked yet
    Restricted,     // System policy prevents access (e.g., MDM)
    Unknown,        // Cannot determine status (error or unsupported)
}
```

### Status Flow

```
NotDetermined ──request()──> [User Prompt] ──yes──> Granted
                                    │
                                    └──no──> Denied

Denied ──open_settings()──> System Settings ──user changes──> Granted

Restricted ──(cannot change)──> Restricted
```

## File Structure

```
src-tauri/src/permissions/
├── mod.rs                  # Public API + permission registry
├── contacts.rs             # CNContactStore (Contacts.framework)
├── calendar.rs             # EKEventStore for calendar (EventKit)
├── reminders.rs            # EKEventStore for reminders (EventKit)
├── location.rs             # CLLocationManager (CoreLocation)
├── camera.rs               # AVCaptureDevice (AVFoundation)
├── microphone.rs           # AVCaptureDevice (AVFoundation)
├── photos.rs               # PHPhotoLibrary (Photos.framework)
├── screen_recording.rs     # CGPreflightScreenCaptureAccess
├── accessibility.rs        # AXIsProcessTrusted
├── automation.rs           # AppleEvents (AEDeterminePermissionToAutomateTarget)
└── full_disk.rs            # File system test (~/.ssh/)
```

## Key Implementations

### `mod.rs` — Permission Registry

**Lines:** ~300

**Key Type:**

```rust
pub struct Permission {
    pub id: String,              // e.g., "contacts"
    pub name: String,            // "Contacts"
    pub description: String,     // "Access your contacts..."
    pub status: PermissionStatus,
    pub required: bool,          // Is this required for core functionality?
    pub settings_url: Option<String>, // Deep link to System Settings
}
```

**Permission IDs:**

```rust
pub mod ids {
    pub const CONTACTS: &str = "contacts";
    pub const CALENDAR: &str = "calendar";
    pub const REMINDERS: &str = "reminders";
    pub const LOCATION: &str = "location";
    pub const PHOTOS: &str = "photos";
    pub const CAMERA: &str = "camera";
    pub const MICROPHONE: &str = "microphone";
    pub const SCREEN_RECORDING: &str = "screen_recording";
    pub const ACCESSIBILITY: &str = "accessibility";
    pub const FULL_DISK_ACCESS: &str = "full_disk_access";
    pub const AUTOMATION: &str = "automation";
}
```

**Tauri Commands:**

```rust
#[tauri::command]
pub fn check_permission(id: &str) -> Result<Permission, String>

#[tauri::command]
pub fn check_all_permissions() -> Vec<Permission>

#[tauri::command]
pub fn request_permission(id: &str) -> Result<Permission, String>

#[tauri::command]
pub fn open_permission_settings(id: &str) -> Result<(), String>
```

### `contacts.rs` — Contacts Permission

**Framework:** `Contacts.framework`

**FFI:**

```rust
use objc2::rc::Retained;
use objc2::ClassType;
use objc2_contacts::{CNAuthorizationStatus, CNContactStore, CNEntityType};

pub fn check() -> PermissionStatus {
    let status = unsafe {
        CNContactStore::authorizationStatusForEntityType(CNEntityType::Contacts)
    };
    
    match status {
        CNAuthorizationStatus::Authorized => PermissionStatus::Granted,
        CNAuthorizationStatus::Denied => PermissionStatus::Denied,
        CNAuthorizationStatus::NotDetermined => PermissionStatus::NotDetermined,
        CNAuthorizationStatus::Restricted => PermissionStatus::Restricted,
        _ => PermissionStatus::Unknown,
    }
}

pub fn request() -> PermissionStatus {
    let store = unsafe { CNContactStore::new() };
    
    // Request access (async in Objective-C, blocking here)
    let (tx, rx) = std::sync::mpsc::channel();
    
    unsafe {
        store.requestAccessForEntityType_completionHandler(
            CNEntityType::Contacts,
            &block::ConcreteBlock::new(move |granted: bool, _error| {
                let _ = tx.send(granted);
            }).copy(),
        );
    }
    
    // Block until user responds to prompt
    match rx.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(true) => PermissionStatus::Granted,
        Ok(false) => PermissionStatus::Denied,
        Err(_) => PermissionStatus::Unknown, // Timeout
    }
}
```

**Pattern:**
1. Check current status with class method
2. If NotDetermined, call instance method to request
3. Block on callback (convert async Objective-C to sync Rust)
4. Return updated status

### `calendar.rs` — Calendar/Reminders Permission

**Framework:** `EventKit.framework`

**Implementation:**

```rust
use objc2_eventkit::{EKAuthorizationStatus, EKEntityType, EKEventStore};

pub fn check_calendar() -> PermissionStatus {
    let status = unsafe {
        EKEventStore::authorizationStatusForEntityType(EKEntityType::Event)
    };
    convert_status(status)
}

pub fn check_reminders() -> PermissionStatus {
    let status = unsafe {
        EKEventStore::authorizationStatusForEntityType(EKEntityType::Reminder)
    };
    convert_status(status)
}

fn convert_status(status: EKAuthorizationStatus) -> PermissionStatus {
    match status {
        EKAuthorizationStatus::Authorized => PermissionStatus::Granted,
        EKAuthorizationStatus::Denied => PermissionStatus::Denied,
        EKAuthorizationStatus::NotDetermined => PermissionStatus::NotDetermined,
        EKAuthorizationStatus::Restricted => PermissionStatus::Restricted,
        _ => PermissionStatus::Unknown,
    }
}
```

**Note:** Calendar and Reminders are separate permissions but use the same `EKEventStore` API with different `EKEntityType` values.

### `location.rs` — Location Permission

**Framework:** `CoreLocation.framework`

**Implementation:**

```rust
use objc2_core_location::{CLAuthorizationStatus, CLLocationManager};

pub fn check() -> PermissionStatus {
    let manager = unsafe { CLLocationManager::new() };
    let status = unsafe { manager.authorizationStatus() };
    
    match status {
        CLAuthorizationStatus::AuthorizedAlways |
        CLAuthorizationStatus::AuthorizedWhenInUse => PermissionStatus::Granted,
        CLAuthorizationStatus::Denied => PermissionStatus::Denied,
        CLAuthorizationStatus::NotDetermined => PermissionStatus::NotDetermined,
        CLAuthorizationStatus::Restricted => PermissionStatus::Restricted,
        _ => PermissionStatus::Unknown,
    }
}

pub fn request() -> PermissionStatus {
    let manager = unsafe { CLLocationManager::new() };
    
    // Request "when in use" authorization
    unsafe {
        manager.requestWhenInUseAuthorization();
    }
    
    // Note: Authorization happens async, status may still be NotDetermined
    // after this call returns. App must observe delegate callbacks.
    check()
}
```

**Gotcha:** Location permission requires `NSLocationWhenInUseUsageDescription` in `Info.plist`:

```xml
<key>NSLocationWhenInUseUsageDescription</key>
<string>Tairseach needs your location to provide location-based services to agents.</string>
```

Without this key, the permission prompt never appears and the app crashes.

### `screen_recording.rs` — Screen Recording Permission

**Framework:** `ScreenCaptureKit` (macOS 12.3+)

**Implementation:**

```rust
use objc2::rc::Retained;
use objc2_foundation::NSString;

pub fn check() -> PermissionStatus {
    // macOS 12.3+: CGPreflightScreenCaptureAccess
    let has_access = unsafe {
        let result: bool = msg_send![class!(CGDisplayStream), 
                                     preflightScreenCaptureAccess];
        result
    };
    
    if has_access {
        PermissionStatus::Granted
    } else {
        // Cannot distinguish Denied vs NotDetermined via API
        PermissionStatus::NotDetermined
    }
}

pub fn request() -> PermissionStatus {
    // macOS 12.3+: CGRequestScreenCaptureAccess
    unsafe {
        let _: () = msg_send![class!(CGDisplayStream), 
                             requestScreenCaptureAccess];
    }
    
    // Check again after request
    check()
}
```

**Gotcha:** Screen Recording permission requires app to be signed and notarized for the prompt to work correctly.

### `accessibility.rs` — Accessibility Permission

**Framework:** `ApplicationServices.framework`

**Implementation:**

```rust
pub fn check() -> PermissionStatus {
    let trusted = unsafe {
        AXIsProcessTrusted()
    };
    
    if trusted {
        PermissionStatus::Granted
    } else {
        // Cannot distinguish Denied vs NotDetermined
        PermissionStatus::NotDetermined
    }
}

pub fn request() -> PermissionStatus {
    // Create options dictionary requesting prompt
    let options = unsafe {
        let key = NSString::from_str("AXTrustedCheckOptionPrompt");
        let value = NSNumber::from_bool(true);
        NSDictionary::dictionaryWithObject_forKey(value, key)
    };
    
    let _ = unsafe {
        AXIsProcessTrustedWithOptions(options)
    };
    
    // Opens System Settings, but status doesn't change immediately
    PermissionStatus::NotDetermined
}
```

**Gotcha:** Accessibility permission can ONLY be granted in System Settings → Privacy & Security → Accessibility. The `request()` function opens System Settings but doesn't trigger a modal prompt.

### `full_disk.rs` — Full Disk Access Permission

**Framework:** None (filesystem test)

**Implementation:**

```rust
pub fn check() -> PermissionStatus {
    // Test access to a protected location
    let test_paths = vec![
        PathBuf::from(std::env::var("HOME").unwrap()).join(".ssh"),
        PathBuf::from("/Library/Application Support/com.apple.TCC/TCC.db"),
    ];
    
    for path in test_paths {
        if let Ok(_) = std::fs::read_dir(&path) {
            return PermissionStatus::Granted;
        }
    }
    
    PermissionStatus::NotDetermined
}

pub fn request() -> PermissionStatus {
    // Cannot programmatically request Full Disk Access
    // User must manually grant in System Settings
    PermissionStatus::NotDetermined
}
```

**Gotcha:** Full Disk Access has NO API. We test by attempting to read a known-protected directory. If readable, FDA is granted. If not readable, could be Denied or NotDetermined (indistinguishable).

### `automation.rs` — Automation Permission

**Framework:** `AE.framework` (AppleEvents)

**Implementation:**

```rust
use objc2::rc::Retained;
use objc2_foundation::{NSString, NSAppleScript};

pub fn check() -> PermissionStatus {
    // Check if we can send AppleEvents to a common target (Finder)
    let script = r#"
        tell application "Finder"
            return name
        end tell
    "#;
    
    let ns_script = unsafe { NSString::from_str(script) };
    let apple_script = unsafe { NSAppleScript::initWithSource(ns_script) };
    
    let mut error: Option<Retained<NSDictionary>> = None;
    let result = unsafe {
        apple_script.executeAndReturnError(&mut error)
    };
    
    if result.is_some() && error.is_none() {
        PermissionStatus::Granted
    } else {
        // Error -1743 = "not allowed to send AppleEvents"
        PermissionStatus::NotDetermined
    }
}

pub fn request() -> PermissionStatus {
    // Attempting to execute an AppleScript triggers the prompt
    check()
}
```

**Gotcha:** Automation permission is per-target-app. Sending events to Finder requires different permission than sending to Mail. Currently we only check Finder.

## Permission Enforcement Pattern

### Handler Permission Gate

**In** `proxy/handlers/mod.rs`:

```rust
fn required_permission(method: &str) -> Option<&'static str> {
    match method {
        "contacts.list" | "contacts.get" | ... => Some("contacts"),
        "calendar.events" | "calendar.createEvent" | ... => Some("calendar"),
        "location.get" => Some("location"),
        // ... all method → permission mappings
        _ => None,
    }
}
```

**In** `HandlerRegistry::handle()`:

```rust
pub async fn handle(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);
    
    // Check if method requires permission
    if let Some(required) = required_permission(&request.method) {
        let status = check_permission_status(required).await;
        
        if status != PermissionStatus::Granted {
            return JsonRpcResponse::permission_denied(
                id,
                required,
                status.as_str(),
            );
        }
    }
    
    // Permission check passed, dispatch to handler
    self.route(request).await
}
```

**Flow:**

```
Request → HandlerRegistry::handle()
            ↓
        required_permission(method) → Some("contacts")
            ↓
        check_permission_status("contacts") → PermissionStatus::Denied
            ↓
        Return JsonRpcResponse::permission_denied()
        (request never reaches handler)
```

## Frontend Integration

### PermissionsView.vue

**Path:** `src/views/PermissionsView.vue`

**Structure:**

```vue
<template>
  <div class="permissions-grid">
    <PermissionCard
      v-for="perm in permissions"
      :key="perm.id"
      :permission="perm"
      @request="requestPermission(perm.id)"
      @open-settings="openSettings(perm.id)"
    />
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const permissions = ref([])

onMounted(async () => {
  permissions.value = await invoke('check_all_permissions')
})

async function requestPermission(id) {
  const updated = await invoke('request_permission', { id })
  // Update UI with new status
  const index = permissions.value.findIndex(p => p.id === id)
  if (index !== -1) {
    permissions.value[index] = updated
  }
}

function openSettings(id) {
  invoke('open_permission_settings', { id })
}
</script>
```

### Status Badge Component

```vue
<template>
  <span :class="badgeClass">{{ statusText }}</span>
</template>

<script setup>
const props = defineProps(['status'])

const badgeClass = computed(() => ({
  'badge-granted': props.status === 'granted',
  'badge-denied': props.status === 'denied',
  'badge-not-determined': props.status === 'not_determined',
  'badge-restricted': props.status === 'restricted',
}))

const statusText = computed(() => {
  switch (props.status) {
    case 'granted': return '✓ Granted'
    case 'denied': return '✗ Denied'
    case 'not_determined': return '? Not Asked'
    case 'restricted': return '⚠ Restricted'
    default: return 'Unknown'
  }
})
</script>
```

## Testing

### Manual Testing

1. **Check status:**
   ```bash
   # Via Tauri devtools
   invoke('check_permission', { id: 'contacts' })
   # Returns: { id: "contacts", status: "not_determined", ... }
   ```

2. **Request permission:**
   ```bash
   invoke('request_permission', { id: 'contacts' })
   # macOS prompt appears
   # Returns updated Permission object
   ```

3. **Open settings:**
   ```bash
   invoke('open_permission_settings', { id: 'contacts' })
   # Opens System Settings → Privacy & Security → Contacts
   ```

### Automated Testing

**Not yet implemented.**

**Challenge:** macOS permission prompts are system-level and cannot be automated easily.

**Future:** Mock permission status for testing without actual OS interaction.

## Common Issues

### Issue: Permission prompt never appears

**Causes:**
1. Missing `Info.plist` key (e.g., `NSContactsUsageDescription`)
2. App not signed (Screen Recording, Accessibility)
3. Permission already denied (prompt only shows once)

**Solution:**
1. Check `Info.plist` for required keys
2. Sign app with Developer ID
3. Reset permission in System Settings → Privacy & Security

### Issue: Permission status stuck in NotDetermined

**Cause:** User closed prompt without responding

**Solution:**
- Call `request_permission()` again
- Or direct user to System Settings via `open_permission_settings()`

### Issue: Accessibility permission always shows NotDetermined

**Cause:** Accessibility permission doesn't have a modal prompt, only System Settings

**Solution:**
- Call `open_permission_settings('accessibility')`
- User must manually enable in System Settings

### Issue: Full Disk Access shows Denied but files are accessible

**Cause:** FDA check is heuristic-based (file read test), not API-based

**Solution:**
- Ignore status if actual file operations succeed
- Or improve heuristic with better test paths

## Related Documentation

- **[handlers.md](handlers.md)** — How handlers use permission gates
- **[core-server.md](core-server.md)** — Permission enforcement in request lifecycle

---

*The threshold requires permission to cross. Ask first.*

🌬️ **Senchán Torpéist**
