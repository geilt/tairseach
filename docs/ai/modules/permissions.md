# Permissions Module

> **Location:** `src-tauri/src/permissions/`  
> **Files:** 12  
> **Lines:** 1,085  
> **Purpose:** macOS TCC (Transparency, Consent, and Control) permission management

---

## Overview

The Permissions module provides a unified interface for checking, requesting, and managing macOS system permissions. It wraps native TCC APIs and provides both synchronous status checks and asynchronous request flows with user callbacks.

**Supported Permissions:**
- Contacts
- Calendar
- Reminders
- Photos
- Camera
- Microphone
- Location
- Screen Recording
- Accessibility
- Full Disk Access
- Automation (AppleEvents)

---

## File Listing

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | ~520 | Public API, status enum, permission definitions, Tauri commands |
| `accessibility.rs` | ~50 | AXIsProcessTrusted check |
| `automation.rs` | ~45 | AppleEvents permission (macOS 10.14+) |
| `calendar.rs` | ~55 | EventKit calendar permission |
| `camera.rs` | ~50 | AVFoundation camera permission |
| `contacts.rs` | ~55 | AddressBook/Contacts permission |
| `full_disk.rs` | ~100 | Full Disk Access check (via sentinel file test) |
| `location.rs` | ~80 | Core Location permission |
| `microphone.rs` | ~50 | AVFoundation microphone permission |
| `photos.rs` | ~50 | Photos library permission |
| `reminders.rs` | ~55 | EventKit reminders permission |
| `screen_recording.rs` | ~60 | Screen Recording permission (macOS 10.15+) |

---

## Key Types

```rust
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
    Restricted,
    Unknown,
}

pub struct Permission {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: PermissionStatus,
    pub critical: bool,
    pub last_checked: Option<String>,  // ISO 8601 timestamp
}

pub struct PermissionDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub critical: bool,
    pub icon: String,
    pub system_pref_pane: String,
}
```

---

## Permission IDs

```rust
pub mod ids {
    pub const CONTACTS: &str = "contacts";
    pub const AUTOMATION: &str = "automation";
    pub const FULL_DISK_ACCESS: &str = "full_disk_access";
    pub const ACCESSIBILITY: &str = "accessibility";
    pub const SCREEN_RECORDING: &str = "screen_recording";
    pub const CALENDAR: &str = "calendar";
    pub const REMINDERS: &str = "reminders";
    pub const PHOTOS: &str = "photos";
    pub const CAMERA: &str = "camera";
    pub const MICROPHONE: &str = "microphone";
    pub const LOCATION: &str = "location";
}
```

---

## Core APIs

### Checking Permissions

```rust
pub fn check_permission(id: &str) -> Result<Permission, String>

pub fn check_all_permissions() -> Vec<Permission>
```

**Example:**
```rust
let perm = check_permission(ids::CONTACTS)?;

match perm.status {
    PermissionStatus::Granted => println!("Access granted"),
    PermissionStatus::Denied => println!("Access denied"),
    PermissionStatus::NotDetermined => println!("Not yet asked"),
    _ => println!("Unknown status"),
}
```

---

### Requesting Permissions

```rust
pub fn request_permission_with_callback<F>(
    id: &str,
    callback: F,
) -> Result<(), String>
where
    F: Fn(PermissionStatus) + Send + 'static
```

**Example:**
```rust
request_permission_with_callback(ids::CONTACTS, |status| {
    println!("Contacts permission status: {:?}", status);
})?;
```

**Note:** Some permissions (Accessibility, Full Disk Access, Screen Recording) cannot be programmatically requested — users must manually enable them in System Preferences.

---

### Opening System Preferences

```rust
pub fn open_permission_settings(id: &str) -> Result<(), String>
```

**Example:**
```rust
// Opens System Preferences → Privacy & Security → Accessibility
open_permission_settings(ids::ACCESSIBILITY)?;
```

**Pref Pane URLs:**
```rust
match id {
    ids::ACCESSIBILITY => "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
    ids::FULL_DISK_ACCESS => "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles",
    ids::SCREEN_RECORDING => "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
    ids::CONTACTS => "x-apple.systempreferences:com.apple.preference.security?Privacy_Contacts",
    // ...
}
```

---

## Tauri Commands

Exposed to Vue frontend:

```rust
#[tauri::command]
pub fn get_permissions() -> Vec<Permission>

#[tauri::command]
pub fn check_permission(id: String) -> Result<Permission, String>

#[tauri::command]
pub fn check_all_permissions() -> Vec<Permission>

#[tauri::command]
pub async fn request_permission(
    id: String,
    window: tauri::Window,
) -> Result<Permission, String>

#[tauri::command]
pub fn grant_permission(id: String) -> Result<Permission, String>

#[tauri::command]
pub fn revoke_permission(id: String) -> Result<Permission, String>

#[tauri::command]
pub fn open_permission_settings(id: String) -> Result<(), String>

#[tauri::command]
pub fn get_permission_definitions() -> Vec<PermissionDefinition>
```

---

## Permission Definitions

```rust
pub fn get_permission_definitions() -> Vec<PermissionDefinition> {
    vec![
        PermissionDefinition {
            id: ids::CONTACTS.to_string(),
            name: "Contacts".to_string(),
            description: "Access to your contacts for agent context and automation.".to_string(),
            critical: true,
            icon: "person-circle".to_string(),
            system_pref_pane: "Privacy_Contacts".to_string(),
        },
        PermissionDefinition {
            id: ids::CALENDAR.to_string(),
            name: "Calendar".to_string(),
            description: "Access to your calendar for scheduling and reminders.".to_string(),
            critical: true,
            icon: "calendar".to_string(),
            system_pref_pane: "Privacy_Calendars".to_string(),
        },
        // ... 9 more
    ]
}
```

---

## Platform-Specific Checks

### Accessibility (AXIsProcessTrusted)

```rust
// accessibility.rs
#[cfg(target_os = "macos")]
pub fn check() -> PermissionStatus {
    use core_foundation::base::TCFType;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;
    
    unsafe {
        let options = CFDictionary::from_CFType_pairs(&[(
            CFString::new("AXTrustedCheckOptionPrompt"),
            kCFBooleanFalse,
        )]);
        
        if AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) {
            PermissionStatus::Granted
        } else {
            PermissionStatus::Denied
        }
    }
}
```

---

### Full Disk Access (Sentinel File Test)

```rust
// full_disk.rs
pub fn check() -> PermissionStatus {
    // Try to read a protected file that requires FDA
    let test_path = dirs::home_dir()
        .unwrap()
        .join("Library/Safari/CloudTabs.db");
    
    match std::fs::metadata(&test_path) {
        Ok(_) => PermissionStatus::Granted,
        Err(_) => PermissionStatus::Denied,
    }
}
```

**Why:** macOS doesn't provide a direct API to check FDA status, so we test read access to a known protected file.

---

### Contacts (CNContactStore)

```rust
// contacts.rs
#[cfg(target_os = "macos")]
pub fn check() -> PermissionStatus {
    unsafe {
        let store = CNContactStore::new();
        match store.authorizationStatus() {
            CNAuthorizationStatus::Authorized => PermissionStatus::Granted,
            CNAuthorizationStatus::Denied => PermissionStatus::Denied,
            CNAuthorizationStatus::Restricted => PermissionStatus::Restricted,
            CNAuthorizationStatus::NotDetermined => PermissionStatus::NotDetermined,
        }
    }
}

pub fn request<F>(callback: F) -> Result<(), String>
where
    F: Fn(PermissionStatus) + Send + 'static,
{
    unsafe {
        let store = CNContactStore::new();
        store.requestAccess(|granted, error| {
            let status = if granted {
                PermissionStatus::Granted
            } else {
                PermissionStatus::Denied
            };
            callback(status);
        });
    }
    Ok(())
}
```

---

## Shared Helpers

### TCC Check Pattern

All TCC-based permissions follow this pattern:

```rust
pub fn check() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        unsafe {
            match AuthorizationStatus::for_permission(PERMISSION_TYPE) {
                0 => PermissionStatus::NotDetermined,
                1 => PermissionStatus::Restricted,
                2 => PermissionStatus::Denied,
                3 => PermissionStatus::Granted,
                _ => PermissionStatus::Unknown,
            }
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    PermissionStatus::Unknown
}
```

---

## Usage in Handlers

Handlers don't call this module directly — permission enforcement is handled by `HandlerRegistry`:

```rust
// In proxy/handlers/mod.rs
if let Some(required) = required_permission(&request.method) {
    let status = check_permission_status(required).await;
    
    if status != PermissionStatus::Granted {
        return JsonRpcResponse::permission_denied(id, required, status.as_str());
    }
}
```

---

## Frontend Integration

Vue components use Tauri commands:

```typescript
import { invoke } from '@/api'

// Check all permissions
const perms = await invoke('get_permissions')

// Request specific permission
const perm = await invoke('request_permission', { id: 'contacts' })

// Open System Preferences
await invoke('open_permission_settings', { id: 'accessibility' })
```

---

## Critical vs Optional

```rust
pub fn is_critical(id: &str) -> bool {
    matches!(
        id,
        ids::CONTACTS
            | ids::CALENDAR
            | ids::FULL_DISK_ACCESS
            | ids::ACCESSIBILITY
            | ids::SCREEN_RECORDING
    )
}
```

**Critical permissions** block core functionality.  
**Optional permissions** enhance but aren't required.

---

## Recent Refactorings

**Branch:** `refactor/permissions-dry` (merged 2026-02-13)

**Changes:**
- Extracted shared TCC check pattern
- Added `request_permission_with_callback()` helper
- Standardized permission definitions
- Removed duplicate status conversion logic

---

*For handler permission enforcement, see [handlers.md](handlers.md)*
