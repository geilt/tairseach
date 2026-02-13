//! Contacts Handler
//!
//! Handles contact-related JSON-RPC methods using native CNContactStore.
//! This runs WITHIN Tairseach's process, using its granted permissions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use super::common::*;
use super::super::protocol::JsonRpcResponse;

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

/// Handle contact-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "list" => handle_list(params, id).await,
        "search" => handle_search(params, id).await,
        "get" => handle_get(params, id).await,
        "create" => handle_create(params, id).await,
        "update" => handle_update(params, id).await,
        "delete" => handle_delete(params, id).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("contacts.{}", action)),
    }
}

/// List all contacts with optional pagination
async fn handle_list(params: &Value, id: Value) -> JsonRpcResponse {
    let limit = u64_with_default(params, "limit", 100) as usize;
    let offset = u64_with_default(params, "offset", 0) as usize;
    
    match fetch_contacts_native(None, limit, offset) {
        Ok(contacts) => ok(
            id,
            serde_json::json!({
                "contacts": contacts,
                "count": contacts.len(),
                "limit": limit,
                "offset": offset,
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

/// Search contacts by name
async fn handle_search(params: &Value, id: Value) -> JsonRpcResponse {
    let query = match require_string(params, "query", &id) {
        Ok(q) => q,
        Err(response) => return response,
    };
    
    let limit = u64_with_default(params, "limit", 50) as usize;
    
    match fetch_contacts_native(Some(query), limit, 0) {
        Ok(contacts) => ok(
            id,
            serde_json::json!({
                "query": query,
                "contacts": contacts,
                "count": contacts.len(),
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

/// Get a specific contact by ID
async fn handle_get(params: &Value, id: Value) -> JsonRpcResponse {
    let contact_id = match require_string(params, "id", &id) {
        Ok(c) => c,
        Err(response) => return response,
    };
    
    match fetch_contact_by_id_native(contact_id) {
        Ok(Some(contact)) => ok(id, serde_json::to_value(contact).unwrap_or_default()),
        Ok(None) => error(id, -32002, format!("Contact not found: {}", contact_id)),
        Err(e) => generic_error(id, e),
    }
}

/// Create a new contact
async fn handle_create(params: &Value, id: Value) -> JsonRpcResponse {
    let first_name = optional_string(params, "firstName");
    let last_name = optional_string(params, "lastName");
    let organization = optional_string(params, "organization");
    let emails = optional_string_array(params, "emails").unwrap_or_default();
    let phones = optional_string_array(params, "phones").unwrap_or_default();
    
    if first_name.is_none() && last_name.is_none() && organization.is_none() {
        return invalid_params(
            id,
            "At least one of 'firstName', 'lastName', or 'organization' is required",
        );
    }
    
    match create_contact_native(first_name, last_name, organization, &emails, &phones) {
        Ok(contact) => ok(
            id,
            serde_json::json!({
                "created": true,
                "contact": contact,
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

/// Update an existing contact
async fn handle_update(params: &Value, id: Value) -> JsonRpcResponse {
    let contact_id = match require_string(params, "id", &id) {
        Ok(c) => c,
        Err(response) => return response,
    };
    
    let first_name = optional_string(params, "firstName");
    let last_name = optional_string(params, "lastName");
    let organization = optional_string(params, "organization");
    let emails = optional_string_array(params, "emails");
    let phones = optional_string_array(params, "phones");
    
    match update_contact_native(contact_id, first_name, last_name, organization, emails.as_deref(), phones.as_deref()) {
        Ok(contact) => ok(
            id,
            serde_json::json!({
                "updated": true,
                "contact": contact,
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

/// Delete a contact
async fn handle_delete(params: &Value, id: Value) -> JsonRpcResponse {
    let contact_id = match require_string(params, "id", &id) {
        Ok(c) => c,
        Err(response) => return response,
    };
    
    match delete_contact_native(contact_id) {
        Ok(()) => ok(
            id,
            serde_json::json!({
                "deleted": true,
                "id": contact_id,
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

// ============================================================================
// Native CNContactStore integration
// ============================================================================

#[cfg(target_os = "macos")]
fn fetch_contacts_native(search: Option<&str>, limit: usize, offset: usize) -> Result<Vec<Contact>, String> {
    use objc2_contacts::CNContactStore;
    use objc2_foundation::{NSArray, NSPredicate, NSString};
    
    info!("Fetching contacts natively: search={:?}, limit={}, offset={}", search, limit, offset);
    
    unsafe {
        let store = CNContactStore::new();
        
        // Create predicate
        let predicate = if let Some(query) = search {
            let ns_query = NSString::from_str(query);
            let format = NSString::from_str("givenName CONTAINS[cd] %@ OR familyName CONTAINS[cd] %@");
            let args: objc2::rc::Retained<NSArray<NSString>> = NSArray::from_slice(&[&*ns_query, &*ns_query]);
            let args_untyped: &NSArray = &*(objc2::rc::Retained::as_ptr(&args) as *const NSArray);
            NSPredicate::predicateWithFormat_argumentArray(&format, Some(args_untyped))
        } else {
            let format = NSString::from_str("TRUEPREDICATE");
            NSPredicate::predicateWithFormat_argumentArray(&format, None)
        };
        
        // Keys to fetch
        let keys = get_contact_keys();
        let keys_cast: &NSArray<objc2::runtime::ProtocolObject<dyn objc2_contacts::CNKeyDescriptor>> = 
            &*(objc2::rc::Retained::as_ptr(&keys) as *const _);
        
        // Fetch contacts
        let result = store.unifiedContactsMatchingPredicate_keysToFetch_error(&predicate, keys_cast);
        let contacts_array = result.map_err(|e| format!("CNContactStore error: {}", e.localizedDescription()))?;
        
        // Convert to our Contact struct
        let mut contacts = Vec::new();
        let count = contacts_array.count();
        let end = (offset + limit).min(count);
        
        for i in offset..end {
            let cn_contact = contacts_array.objectAtIndex(i);
            contacts.push(cn_contact_to_contact(&cn_contact));
        }
        
        info!("Fetched {} contacts (total: {})", contacts.len(), count);
        Ok(contacts)
    }
}

#[cfg(target_os = "macos")]
fn fetch_contact_by_id_native(contact_id: &str) -> Result<Option<Contact>, String> {
    use objc2_contacts::CNContactStore;
    use objc2_foundation::{NSArray, NSString};
    
    info!("Fetching contact by ID: {}", contact_id);
    
    unsafe {
        let store = CNContactStore::new();
        let identifier = NSString::from_str(contact_id);
        
        let keys = get_contact_keys();
        let keys_cast: &NSArray<objc2::runtime::ProtocolObject<dyn objc2_contacts::CNKeyDescriptor>> = 
            &*(objc2::rc::Retained::as_ptr(&keys) as *const _);
        
        match store.unifiedContactWithIdentifier_keysToFetch_error(&identifier, keys_cast) {
            Ok(cn_contact) => Ok(Some(cn_contact_to_contact(&cn_contact))),
            Err(e) => {
                let desc = e.localizedDescription().to_string();
                if desc.contains("not found") || desc.contains("No contacts") {
                    Ok(None)
                } else {
                    Err(format!("CNContactStore error: {}", desc))
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn create_contact_native(
    first_name: Option<&str>,
    last_name: Option<&str>,
    organization: Option<&str>,
    emails: &[String],
    phones: &[String],
) -> Result<Contact, String> {
    use objc2_contacts::{CNContactStore, CNLabeledValue, CNMutableContact, CNPhoneNumber, CNSaveRequest};
    use objc2_foundation::{NSArray, NSString};
    
    info!("Creating contact: firstName={:?}, lastName={:?}", first_name, last_name);
    
    unsafe {
        let store = CNContactStore::new();
        let contact = CNMutableContact::new();
        
        // Set basic fields
        if let Some(fn_str) = first_name {
            contact.setGivenName(&NSString::from_str(fn_str));
        }
        if let Some(ln_str) = last_name {
            contact.setFamilyName(&NSString::from_str(ln_str));
        }
        if let Some(org_str) = organization {
            contact.setOrganizationName(&NSString::from_str(org_str));
        }
        
        // Set email addresses
        if !emails.is_empty() {
            let mut email_values: Vec<objc2::rc::Retained<CNLabeledValue<NSString>>> = Vec::new();
            for email in emails {
                let label = NSString::from_str("_$!<Home>!$_");
                let value = NSString::from_str(email);
                // Dereference both label and value to get &NSString
                let labeled = CNLabeledValue::labeledValueWithLabel_value(Some(&*label), &*value);
                email_values.push(labeled);
            }
            
            let email_refs: Vec<&CNLabeledValue<NSString>> = email_values.iter().map(|v| &**v).collect();
            let email_array = NSArray::from_slice(&email_refs);
            contact.setEmailAddresses(&email_array);
        }
        
        // Set phone numbers
        if !phones.is_empty() {
            let mut phone_values: Vec<objc2::rc::Retained<CNLabeledValue<CNPhoneNumber>>> = Vec::new();
            for phone in phones {
                let label = NSString::from_str("_$!<Mobile>!$_");
                let phone_str = NSString::from_str(phone);
                if let Some(phone_number) = CNPhoneNumber::phoneNumberWithStringValue(&*phone_str) {
                    // Dereference label and phone_number to get &NSString and &CNPhoneNumber
                    let labeled = CNLabeledValue::labeledValueWithLabel_value(Some(&*label), &*phone_number);
                    phone_values.push(labeled);
                }
            }
            
            let phone_refs: Vec<&CNLabeledValue<CNPhoneNumber>> = phone_values.iter().map(|v| &**v).collect();
            let phone_array = NSArray::from_slice(&phone_refs);
            contact.setPhoneNumbers(&phone_array);
        }
        
        // Create save request and add contact
        let save_request = CNSaveRequest::new();
        save_request.addContact_toContainerWithIdentifier(&contact, None);
        
        // Execute save
        store.executeSaveRequest_error(&save_request)
            .map_err(|e| format!("Failed to save contact: {}", e.localizedDescription()))?;
        
        // Get the new ID and return the contact
        let new_id = contact.identifier().to_string();
        info!("Created contact with ID: {}", new_id);
        
        // Build full name
        let fn_val = first_name.map(String::from);
        let ln_val = last_name.map(String::from);
        let full_name = match (&fn_val, &ln_val) {
            (Some(f), Some(l)) => Some(format!("{} {}", f, l)),
            (Some(f), None) => Some(f.clone()),
            (None, Some(l)) => Some(l.clone()),
            (None, None) => None,
        };
        
        Ok(Contact {
            id: new_id,
            first_name: fn_val,
            last_name: ln_val,
            full_name,
            emails: emails.to_vec(),
            phones: phones.to_vec(),
            organization: organization.map(String::from),
        })
    }
}

#[cfg(target_os = "macos")]
fn update_contact_native(
    contact_id: &str,
    first_name: Option<&str>,
    last_name: Option<&str>,
    organization: Option<&str>,
    emails: Option<&[String]>,
    phones: Option<&[String]>,
) -> Result<Contact, String> {
    use objc2_contacts::{CNContactStore, CNLabeledValue, CNMutableContact, CNPhoneNumber, CNSaveRequest};
    use objc2_foundation::{NSArray, NSMutableCopying, NSString};
    
    info!("Updating contact: id={}", contact_id);
    
    unsafe {
        let store = CNContactStore::new();
        let identifier = NSString::from_str(contact_id);
        
        // Fetch existing contact with all keys we need to update
        let keys = get_contact_keys();
        let keys_cast: &NSArray<objc2::runtime::ProtocolObject<dyn objc2_contacts::CNKeyDescriptor>> = 
            &*(objc2::rc::Retained::as_ptr(&keys) as *const _);
        
        let cn_contact = store.unifiedContactWithIdentifier_keysToFetch_error(&identifier, keys_cast)
            .map_err(|e| format!("Contact not found: {}", e.localizedDescription()))?;
        
        // Create mutable copy
        let mutable_contact: objc2::rc::Retained<CNMutableContact> = cn_contact.mutableCopy();
        
        // Update fields if provided
        if let Some(fn_str) = first_name {
            mutable_contact.setGivenName(&NSString::from_str(fn_str));
        }
        if let Some(ln_str) = last_name {
            mutable_contact.setFamilyName(&NSString::from_str(ln_str));
        }
        if let Some(org_str) = organization {
            mutable_contact.setOrganizationName(&NSString::from_str(org_str));
        }
        
        // Update emails if provided
        if let Some(email_list) = emails {
            let mut email_values: Vec<objc2::rc::Retained<CNLabeledValue<NSString>>> = Vec::new();
            for email in email_list {
                let label = NSString::from_str("_$!<Home>!$_");
                let value = NSString::from_str(email);
                // Dereference both label and value to get &NSString
                let labeled = CNLabeledValue::labeledValueWithLabel_value(Some(&*label), &*value);
                email_values.push(labeled);
            }
            
            let email_refs: Vec<&CNLabeledValue<NSString>> = email_values.iter().map(|v| &**v).collect();
            let email_array = NSArray::from_slice(&email_refs);
            mutable_contact.setEmailAddresses(&email_array);
        }
        
        // Update phones if provided
        if let Some(phone_list) = phones {
            let mut phone_values: Vec<objc2::rc::Retained<CNLabeledValue<CNPhoneNumber>>> = Vec::new();
            for phone in phone_list {
                let label = NSString::from_str("_$!<Mobile>!$_");
                let phone_str = NSString::from_str(phone);
                if let Some(phone_number) = CNPhoneNumber::phoneNumberWithStringValue(&*phone_str) {
                    // Dereference label and phone_number to get &NSString and &CNPhoneNumber
                    let labeled = CNLabeledValue::labeledValueWithLabel_value(Some(&*label), &*phone_number);
                    phone_values.push(labeled);
                }
            }
            
            let phone_refs: Vec<&CNLabeledValue<CNPhoneNumber>> = phone_values.iter().map(|v| &**v).collect();
            let phone_array = NSArray::from_slice(&phone_refs);
            mutable_contact.setPhoneNumbers(&phone_array);
        }
        
        // Create save request and update contact
        let save_request = CNSaveRequest::new();
        save_request.updateContact(&mutable_contact);
        
        // Execute save
        store.executeSaveRequest_error(&save_request)
            .map_err(|e| format!("Failed to update contact: {}", e.localizedDescription()))?;
        
        info!("Updated contact: {}", contact_id);
        
        // Re-fetch to get updated values
        fetch_contact_by_id_native(contact_id)?
            .ok_or_else(|| "Contact disappeared after update".to_string())
    }
}

#[cfg(target_os = "macos")]
fn delete_contact_native(contact_id: &str) -> Result<(), String> {
    use objc2_contacts::{CNContactStore, CNSaveRequest};
    use objc2_foundation::{NSArray, NSMutableCopying, NSString};
    
    info!("Deleting contact: id={}", contact_id);
    
    unsafe {
        let store = CNContactStore::new();
        let identifier = NSString::from_str(contact_id);
        
        // Fetch existing contact (we need minimal keys for deletion)
        let key_id = NSString::from_str("identifier");
        let keys = NSArray::from_slice(&[&*key_id]);
        let keys_cast: &NSArray<objc2::runtime::ProtocolObject<dyn objc2_contacts::CNKeyDescriptor>> = 
            &*(objc2::rc::Retained::as_ptr(&keys) as *const _);
        
        let cn_contact = store.unifiedContactWithIdentifier_keysToFetch_error(&identifier, keys_cast)
            .map_err(|e| format!("Contact not found: {}", e.localizedDescription()))?;
        
        // Create mutable copy for deletion
        let mutable_contact: objc2::rc::Retained<objc2_contacts::CNMutableContact> = cn_contact.mutableCopy();
        
        // Create save request and delete contact
        let save_request = CNSaveRequest::new();
        save_request.deleteContact(&mutable_contact);
        
        // Execute save
        store.executeSaveRequest_error(&save_request)
            .map_err(|e| format!("Failed to delete contact: {}", e.localizedDescription()))?;
        
        info!("Deleted contact: {}", contact_id);
        Ok(())
    }
}

/// Helper: Get standard contact fetch keys
#[cfg(target_os = "macos")]
fn get_contact_keys() -> objc2::rc::Retained<objc2_foundation::NSArray<objc2_foundation::NSString>> {
    use objc2_foundation::{NSArray, NSString};
    
    let key_id = NSString::from_str("identifier");
    let key_given = NSString::from_str("givenName");
    let key_family = NSString::from_str("familyName");
    let key_org = NSString::from_str("organizationName");
    let key_emails = NSString::from_str("emailAddresses");
    let key_phones = NSString::from_str("phoneNumbers");
    
    NSArray::from_slice(&[&*key_id, &*key_given, &*key_family, &*key_org, &*key_emails, &*key_phones])
}

/// Helper: Convert CNContact to our Contact struct
#[cfg(target_os = "macos")]
fn cn_contact_to_contact(cn_contact: &objc2_contacts::CNContact) -> Contact {
    unsafe {
        let first_name = cn_contact.givenName().to_string();
        let last_name = cn_contact.familyName().to_string();
        let organization = cn_contact.organizationName().to_string();
        let identifier = cn_contact.identifier().to_string();
        
        let full_name = match (first_name.is_empty(), last_name.is_empty()) {
            (true, true) => None,
            (true, false) => Some(last_name.clone()),
            (false, true) => Some(first_name.clone()),
            (false, false) => Some(format!("{} {}", first_name, last_name)),
        };
        
        let mut emails = Vec::new();
        let email_addresses = cn_contact.emailAddresses();
        for j in 0..email_addresses.count() {
            let labeled = email_addresses.objectAtIndex(j);
            emails.push(labeled.value().to_string());
        }
        
        let mut phones = Vec::new();
        let phone_numbers = cn_contact.phoneNumbers();
        for j in 0..phone_numbers.count() {
            let labeled = phone_numbers.objectAtIndex(j);
            phones.push(labeled.value().stringValue().to_string());
        }
        
        Contact {
            id: identifier,
            first_name: if first_name.is_empty() { None } else { Some(first_name) },
            last_name: if last_name.is_empty() { None } else { Some(last_name) },
            full_name,
            emails,
            phones,
            organization: if organization.is_empty() { None } else { Some(organization) },
        }
    }
}

// ============================================================================
// Non-macOS stubs
// ============================================================================

#[cfg(not(target_os = "macos"))]
fn fetch_contacts_native(_search: Option<&str>, _limit: usize, _offset: usize) -> Result<Vec<Contact>, String> {
    Err("Contacts are only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
fn fetch_contact_by_id_native(_contact_id: &str) -> Result<Option<Contact>, String> {
    Err("Contacts are only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
fn create_contact_native(_: Option<&str>, _: Option<&str>, _: Option<&str>, _: &[String], _: &[String]) -> Result<Contact, String> {
    Err("Contacts are only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
fn update_contact_native(_: &str, _: Option<&str>, _: Option<&str>, _: Option<&str>, _: Option<&[String]>, _: Option<&[String]>) -> Result<Contact, String> {
    Err("Contacts are only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
fn delete_contact_native(_contact_id: &str) -> Result<(), String> {
    Err("Contacts are only available on macOS".to_string())
}
