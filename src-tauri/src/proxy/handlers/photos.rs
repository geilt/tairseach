//! Photos Handler
//!
//! Handles photo library access using PhotoKit (PHPhotoLibrary).
//! Read-only operations: list albums, list photos, get photo metadata.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Album representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub album_type: String,
    #[serde(rename = "assetCount")]
    pub asset_count: u32,
}

/// Photo/Asset representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub id: String,
    #[serde(rename = "creationDate")]
    pub creation_date: Option<String>,
    #[serde(rename = "modificationDate")]
    pub modification_date: Option<String>,
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub duration: Option<f64>,
    #[serde(rename = "isFavorite")]
    pub is_favorite: bool,
    pub location: Option<Location>,
}

/// Location representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

/// Handle photos-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "albums" => handle_albums(params, id).await,
        "list" => handle_list(params, id).await,
        "get" => handle_get(params, id).await,
        "search" => handle_search(params, id).await,
        _ => method_not_found(id, &format!("photos.{}", action)),
    }
}

/// List all photo albums
async fn handle_albums(_params: &Value, id: Value) -> JsonRpcResponse {
    match fetch_albums().await {
        Ok(albums) => ok(
            id,
            serde_json::json!({
                "albums": albums,
                "count": albums.len(),
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

/// List photos from an album (or all photos if no album specified)
async fn handle_list(params: &Value, id: Value) -> JsonRpcResponse {
    let album_id = optional_string(params, "albumId");
    let limit = u64_with_default(params, "limit", 100) as usize;
    let offset = u64_with_default(params, "offset", 0) as usize;
    
    match fetch_photos(album_id, limit, offset).await {
        Ok(photos) => ok(
            id,
            serde_json::json!({
                "photos": photos,
                "count": photos.len(),
                "limit": limit,
                "offset": offset,
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

/// Get a specific photo by ID
async fn handle_get(params: &Value, id: Value) -> JsonRpcResponse {
    let photo_id = match require_string(params, "id", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };
    
    match fetch_photo_by_id(photo_id).await {
        Ok(Some(photo)) => ok(id, serde_json::to_value(photo).unwrap_or_default()),
        Ok(None) => error(id, -32002, format!("Photo not found: {}", photo_id)),
        Err(e) => generic_error(id, e),
    }
}

/// Search photos by query string
async fn handle_search(params: &Value, id: Value) -> JsonRpcResponse {
    let query = match require_string(params, "query", &id) {
        Ok(q) => q,
        Err(response) => return response,
    };
    let limit = u64_with_default(params, "limit", 50) as usize;
    
    match search_photos(query, limit).await {
        Ok(photos) => ok(
            id,
            serde_json::json!({
                "query": query,
                "photos": photos,
                "count": photos.len(),
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

// ============================================================================
// Native PhotoKit integration via JXA
// ============================================================================

/// Fetch all photo albums using JXA
#[cfg(target_os = "macos")]
async fn fetch_albums() -> Result<Vec<Album>, String> {
    use std::process::Command;
    
    info!("Fetching photo albums via JXA");
    
    let script = r#"
        ObjC.import('Photos');
        
        var library = $.PHPhotoLibrary.sharedPhotoLibrary;
        
        // Fetch all albums
        var fetchOptions = $.PHFetchOptions.alloc.init;
        var collections = $.PHAssetCollection.fetchAssetCollectionsWithTypeSubtypeOptions(
            $.PHAssetCollectionTypeAlbum,
            $.PHAssetCollectionSubtypeAny,
            fetchOptions
        );
        
        var result = [];
        
        for (var i = 0; i < collections.count; i++) {
            var collection = collections.objectAtIndex(i);
            var assetFetchOptions = $.PHFetchOptions.alloc.init;
            var assets = $.PHAsset.fetchAssetsInAssetCollectionOptions(collection, assetFetchOptions);
            
            result.push({
                id: ObjC.unwrap(collection.localIdentifier),
                title: ObjC.unwrap(collection.localizedTitle) || 'Untitled',
                type: 'album',
                assetCount: assets.count
            });
        }
        
        // Add smart albums
        var smartAlbums = $.PHAssetCollection.fetchAssetCollectionsWithTypeSubtypeOptions(
            $.PHAssetCollectionTypeSmartAlbum,
            $.PHAssetCollectionSubtypeAny,
            fetchOptions
        );
        
        for (var i = 0; i < smartAlbums.count; i++) {
            var collection = smartAlbums.objectAtIndex(i);
            var assetFetchOptions = $.PHFetchOptions.alloc.init;
            var assets = $.PHAsset.fetchAssetsInAssetCollectionOptions(collection, assetFetchOptions);
            
            result.push({
                id: ObjC.unwrap(collection.localIdentifier),
                title: ObjC.unwrap(collection.localizedTitle) || 'Untitled',
                type: 'smart',
                assetCount: assets.count
            });
        }
        
        JSON.stringify(result);
    "#;
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<Vec<Album>>(stdout.trim())
            .map_err(|e| format!("Failed to parse albums JSON: {}", e))
    } else {
        error!("JXA albums fetch failed: {}", String::from_utf8_lossy(&output.stderr));
        Err(format!("Albums fetch failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}

/// Fetch photos from an album or all photos
#[cfg(target_os = "macos")]
async fn fetch_photos(album_id: Option<&str>, limit: usize, offset: usize) -> Result<Vec<Photo>, String> {
    use std::process::Command;
    
    info!("Fetching photos via JXA: album_id={:?}, limit={}, offset={}", album_id, limit, offset);
    
    let script = format!(
        r#"
        ObjC.import('Photos');
        ObjC.import('Foundation');
        
        var fetchOptions = $.PHFetchOptions.alloc.init;
        fetchOptions.sortDescriptors = $.NSArray.arrayWithObject(
            $.NSSortDescriptor.sortDescriptorWithKeyAscending('creationDate', false)
        );
        
        var assets;
        {fetch_expr}
        
        var result = [];
        var isoFormatter = $.NSDateFormatter.alloc.init;
        isoFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZZZZZ";
        
        var start = {offset};
        var end = Math.min(start + {limit}, assets.count);
        
        for (var i = start; i < end; i++) {{
            var asset = assets.objectAtIndex(i);
            
            var mediaType = 'unknown';
            if (asset.mediaType === $.PHAssetMediaTypeImage) mediaType = 'image';
            else if (asset.mediaType === $.PHAssetMediaTypeVideo) mediaType = 'video';
            else if (asset.mediaType === $.PHAssetMediaTypeAudio) mediaType = 'audio';
            
            var photo = {{
                id: ObjC.unwrap(asset.localIdentifier),
                creationDate: asset.creationDate ? ObjC.unwrap(isoFormatter.stringFromDate(asset.creationDate)) : null,
                modificationDate: asset.modificationDate ? ObjC.unwrap(isoFormatter.stringFromDate(asset.modificationDate)) : null,
                mediaType: mediaType,
                width: asset.pixelWidth,
                height: asset.pixelHeight,
                duration: asset.duration > 0 ? asset.duration : null,
                isFavorite: asset.favorite
            }};
            
            if (asset.location) {{
                photo.location = {{
                    latitude: asset.location.coordinate.latitude,
                    longitude: asset.location.coordinate.longitude
                }};
            }}
            
            result.push(photo);
        }}
        
        JSON.stringify(result);
        "#,
        fetch_expr = if let Some(album_id) = album_id {
            format!(
                r#"
                var collection = $.PHAssetCollection.fetchAssetCollectionsWithLocalIdentifiersOptions(
                    $.NSArray.arrayWithObject('{}'),
                    null
                ).firstObject;
                if (collection) {{
                    assets = $.PHAsset.fetchAssetsInAssetCollectionOptions(collection, fetchOptions);
                }} else {{
                    assets = $.PHAsset.fetchAssetsWithOptions(fetchOptions);
                }}
                "#,
                album_id.replace('\'', "\\'")
            )
        } else {
            "assets = $.PHAsset.fetchAssetsWithOptions(fetchOptions);".to_string()
        },
        limit = limit,
        offset = offset,
    );
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        match serde_json::from_str::<Vec<Photo>>(stdout.trim()) {
            Ok(photos) => {
                info!("Fetched {} photos", photos.len());
                Ok(photos)
            }
            Err(e) => {
                error!("Failed to parse photos JSON: {} — raw: {}", e, &stdout.trim()[..stdout.len().min(200)]);
                Err(format!("Failed to parse photos: {}", e))
            }
        }
    } else {
        error!("JXA photos fetch failed: {}", String::from_utf8_lossy(&output.stderr));
        Err(format!("Photos fetch failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}

/// Fetch a single photo by ID
#[cfg(target_os = "macos")]
async fn fetch_photo_by_id(photo_id: &str) -> Result<Option<Photo>, String> {
    use std::process::Command;
    
    info!("Fetching photo by ID via JXA: {}", photo_id);
    
    let script = format!(
        r#"
        ObjC.import('Photos');
        ObjC.import('Foundation');
        
        var fetchResult = $.PHAsset.fetchAssetsWithLocalIdentifiersOptions(
            $.NSArray.arrayWithObject('{photo_id}'),
            null
        );
        
        if (fetchResult.count === 0) {{
            'null';
        }} else {{
            var asset = fetchResult.firstObject;
            
            var mediaType = 'unknown';
            if (asset.mediaType === $.PHAssetMediaTypeImage) mediaType = 'image';
            else if (asset.mediaType === $.PHAssetMediaTypeVideo) mediaType = 'video';
            else if (asset.mediaType === $.PHAssetMediaTypeAudio) mediaType = 'audio';
            
            var isoFormatter = $.NSDateFormatter.alloc.init;
            isoFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZZZZZ";
            
            var photo = {{
                id: ObjC.unwrap(asset.localIdentifier),
                creationDate: asset.creationDate ? ObjC.unwrap(isoFormatter.stringFromDate(asset.creationDate)) : null,
                modificationDate: asset.modificationDate ? ObjC.unwrap(isoFormatter.stringFromDate(asset.modificationDate)) : null,
                mediaType: mediaType,
                width: asset.pixelWidth,
                height: asset.pixelHeight,
                duration: asset.duration > 0 ? asset.duration : null,
                isFavorite: asset.favorite
            }};
            
            if (asset.location) {{
                photo.location = {{
                    latitude: asset.location.coordinate.latitude,
                    longitude: asset.location.coordinate.longitude
                }};
            }}
            
            JSON.stringify(photo);
        }}
        "#,
        photo_id = photo_id.replace('\'', "\\'")
    );
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout == "null" {
            Ok(None)
        } else {
            serde_json::from_str::<Photo>(&stdout)
                .map(Some)
                .map_err(|e| format!("Failed to parse photo: {}", e))
        }
    } else {
        error!("JXA photo fetch failed: {}", String::from_utf8_lossy(&output.stderr));
        Err(format!("Photo fetch failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}

/// Search photos (currently just filters favorites - full text search would require native implementation)
#[cfg(target_os = "macos")]
async fn search_photos(query: &str, limit: usize) -> Result<Vec<Photo>, String> {
    info!("Searching photos: query={}, limit={}", query, limit);
    
    // For now, treat "favorites" as a special search term
    if query.to_lowercase().contains("favorite") {
        fetch_favorites(limit).await
    } else {
        // Fall back to recent photos for other queries
        // A full implementation would use PHAsset.fetchAssetsWithOptions and predicates
        fetch_photos(None, limit, 0).await
    }
}

/// Fetch favorite photos
#[cfg(target_os = "macos")]
async fn fetch_favorites(limit: usize) -> Result<Vec<Photo>, String> {
    use std::process::Command;
    
    info!("Fetching favorite photos via JXA, limit={}", limit);
    
    let script = format!(
        r#"
        ObjC.import('Photos');
        ObjC.import('Foundation');
        
        var fetchOptions = $.PHFetchOptions.alloc.init;
        fetchOptions.predicate = $.NSPredicate.predicateWithFormat('isFavorite == YES');
        fetchOptions.sortDescriptors = $.NSArray.arrayWithObject(
            $.NSSortDescriptor.sortDescriptorWithKeyAscending('creationDate', false)
        );
        
        var assets = $.PHAsset.fetchAssetsWithOptions(fetchOptions);
        
        var result = [];
        var isoFormatter = $.NSDateFormatter.alloc.init;
        isoFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZZZZZ";
        
        var count = Math.min({limit}, assets.count);
        
        for (var i = 0; i < count; i++) {{
            var asset = assets.objectAtIndex(i);
            
            var mediaType = 'unknown';
            if (asset.mediaType === $.PHAssetMediaTypeImage) mediaType = 'image';
            else if (asset.mediaType === $.PHAssetMediaTypeVideo) mediaType = 'video';
            else if (asset.mediaType === $.PHAssetMediaTypeAudio) mediaType = 'audio';
            
            var photo = {{
                id: ObjC.unwrap(asset.localIdentifier),
                creationDate: asset.creationDate ? ObjC.unwrap(isoFormatter.stringFromDate(asset.creationDate)) : null,
                modificationDate: asset.modificationDate ? ObjC.unwrap(isoFormatter.stringFromDate(asset.modificationDate)) : null,
                mediaType: mediaType,
                width: asset.pixelWidth,
                height: asset.pixelHeight,
                duration: asset.duration > 0 ? asset.duration : null,
                isFavorite: true
            }};
            
            if (asset.location) {{
                photo.location = {{
                    latitude: asset.location.coordinate.latitude,
                    longitude: asset.location.coordinate.longitude
                }};
            }}
            
            result.push(photo);
        }}
        
        JSON.stringify(result);
        "#,
        limit = limit,
    );
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<Vec<Photo>>(stdout.trim())
            .map_err(|e| format!("Failed to parse favorites: {}", e))
    } else {
        error!("JXA favorites fetch failed: {}", String::from_utf8_lossy(&output.stderr));
        Err(format!("Favorites fetch failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}

// Non-macOS stubs
#[cfg(not(target_os = "macos"))]
async fn fetch_albums() -> Result<Vec<Album>, String> {
    Err("Photos are only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
async fn fetch_photos(_album_id: Option<&str>, _limit: usize, _offset: usize) -> Result<Vec<Photo>, String> {
    Err("Photos are only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
async fn fetch_photo_by_id(_photo_id: &str) -> Result<Option<Photo>, String> {
    Err("Photos are only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
async fn search_photos(_query: &str, _limit: usize) -> Result<Vec<Photo>, String> {
    Err("Photos are only available on macOS".to_string())
}
