//! Native Contacts Module
//!
//! Provides direct access to macOS Contacts using objc2 bindings.
//! This runs within Tairseach's process, using its granted permissions.

use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::Bool;
#[cfg(target_os = "macos")]
use objc2_contacts::{
    CNContact, CNContactFetchRequest, CNContactStore, CNEntityType, CNKeyDescriptor,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSError, NSString};

/// Contact representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    #[serde(rename = "fullName")]
    pub full_name: Option<String>,
    pub emails: Vec<String>,
    pub phones: Vec<String>,
    pub organization: Option<String>,
}

/// Fetch all contacts using native CNContactStore
#[cfg(target_os = "macos")]
pub fn fetch_all_contacts(limit: Option<usize>, search: Option<&str>) -> Result<Vec<Contact>, String> {
    use std::process::Command;
    
    tracing::info!("Fetching contacts natively: limit={:?}, search={:?}", limit, search);
    
    // For now, use JXA but run it in a way that inherits Tairseach's permission context
    // This is a workaround until we can properly implement objc2 enumeration with blocks
    
    let limit_val = limit.unwrap_or(100);
    let search_clause = match search {
        Some(q) => format!("'{}'", q.replace('\'', "\\'")),
        None => "null".to_string(),
    };
    
    let script = format!(
        r#"
        ObjC.import('Contacts');
        
        var store = $.CNContactStore.alloc.init;
        var keysToFetch = $.NSArray.arrayWithObjects(
            $.CNContactIdentifierKey,
            $.CNContactGivenNameKey,
            $.CNContactFamilyNameKey,
            $.CNContactOrganizationNameKey,
            $.CNContactEmailAddressesKey,
            $.CNContactPhoneNumbersKey
        );
        
        var request = $.CNContactFetchRequest.alloc.initWithKeysToFetch(keysToFetch);
        var contacts = [];
        var query = {search};
        var limit = {limit};
        
        var error = $();
        store.enumerateContactsWithFetchRequestErrorUsingBlock(request, error, function(contact, stop) {{
            if (contacts.length >= limit) {{
                stop[0] = true;
                return;
            }}
            
            var firstName = ObjC.unwrap(contact.givenName) || '';
            var lastName = ObjC.unwrap(contact.familyName) || '';
            var fullName = [firstName, lastName].filter(Boolean).join(' ');
            
            if (query && fullName.toLowerCase().indexOf(query.toLowerCase()) === -1) {{
                return;
            }}
            
            var emails = [];
            var emailAddresses = contact.emailAddresses;
            for (var i = 0; i < emailAddresses.count; i++) {{
                var labeled = emailAddresses.objectAtIndex(i);
                emails.push(ObjC.unwrap(labeled.value));
            }}
            
            var phones = [];
            var phoneNumbers = contact.phoneNumbers;
            for (var i = 0; i < phoneNumbers.count; i++) {{
                var labeled = phoneNumbers.objectAtIndex(i);
                phones.push(ObjC.unwrap(labeled.value.stringValue));
            }}
            
            contacts.push({{
                id: ObjC.unwrap(contact.identifier),
                firstName: firstName || null,
                lastName: lastName || null,
                fullName: fullName || null,
                organization: ObjC.unwrap(contact.organizationName) || null,
                emails: emails,
                phones: phones
            }});
        }});
        
        JSON.stringify(contacts);
        "#,
        search = search_clause,
        limit = limit_val,
    );
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("JXA script failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let contacts: Vec<Contact> = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Failed to parse contacts JSON: {}", e))?;
    
    tracing::info!("Fetched {} contacts", contacts.len());
    Ok(contacts)
}

#[cfg(not(target_os = "macos"))]
pub fn fetch_all_contacts(_limit: Option<usize>, _search: Option<&str>) -> Result<Vec<Contact>, String> {
    Err("Contacts are only available on macOS".to_string())
}

/// Get a specific contact by ID
#[cfg(target_os = "macos")]
pub fn get_contact_by_id(contact_id: &str) -> Result<Option<Contact>, String> {
    use std::process::Command;
    
    tracing::info!("Fetching contact by ID: {}", contact_id);
    
    let script = format!(
        r#"
        ObjC.import('Contacts');
        
        var store = $.CNContactStore.alloc.init;
        var keysToFetch = $.NSArray.arrayWithObjects(
            $.CNContactIdentifierKey,
            $.CNContactGivenNameKey,
            $.CNContactFamilyNameKey,
            $.CNContactOrganizationNameKey,
            $.CNContactEmailAddressesKey,
            $.CNContactPhoneNumbersKey
        );
        
        var identifier = $.NSString.stringWithString('{id}');
        var error = $();
        var contact = store.unifiedContactWithIdentifierKeysToFetchError(identifier, keysToFetch, error);
        
        if (contact) {{
            var firstName = ObjC.unwrap(contact.givenName) || '';
            var lastName = ObjC.unwrap(contact.familyName) || '';
            var fullName = [firstName, lastName].filter(Boolean).join(' ');
            
            var emails = [];
            var emailAddresses = contact.emailAddresses;
            for (var i = 0; i < emailAddresses.count; i++) {{
                var labeled = emailAddresses.objectAtIndex(i);
                emails.push(ObjC.unwrap(labeled.value));
            }}
            
            var phones = [];
            var phoneNumbers = contact.phoneNumbers;
            for (var i = 0; i < phoneNumbers.count; i++) {{
                var labeled = phoneNumbers.objectAtIndex(i);
                phones.push(ObjC.unwrap(labeled.value.stringValue));
            }}
            
            JSON.stringify({{
                id: ObjC.unwrap(contact.identifier),
                firstName: firstName || null,
                lastName: lastName || null,
                fullName: fullName || null,
                organization: ObjC.unwrap(contact.organizationName) || null,
                emails: emails,
                phones: phones
            }});
        }} else {{
            'null';
        }}
        "#,
        id = contact_id.replace('\'', "\\'")
    );
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("JXA script failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout == "null" {
        Ok(None)
    } else {
        let contact: Contact = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse contact JSON: {}", e))?;
        Ok(Some(contact))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_contact_by_id(_contact_id: &str) -> Result<Option<Contact>, String> {
    Err("Contacts are only available on macOS".to_string())
}

/// Tauri command to list contacts
#[tauri::command]
pub async fn list_contacts(limit: Option<usize>, search: Option<String>) -> Result<Vec<Contact>, String> {
    fetch_all_contacts(limit, search.as_deref())
}

/// Tauri command to get a contact by ID
#[tauri::command]
pub async fn get_contact(id: String) -> Result<Option<Contact>, String> {
    get_contact_by_id(&id)
}
