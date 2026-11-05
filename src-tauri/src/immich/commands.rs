use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

use super::client::{Album, ImmichClient, TimelineMonth};
use super::config::{self, ApiKeyInfo};
use super::{MIN_VERSION, registry, reset_session, resolve, session, sync};
use crate::file_management::ImageFile;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub version: String,
    pub user_name: String,
    pub user_email: String,
}

#[tauri::command]
pub fn immich_get_api_key(app_handle: AppHandle) -> ApiKeyInfo {
    config::load_api_key(&app_handle)
}

#[tauri::command]
pub fn immich_set_api_key(api_key: String, app_handle: AppHandle) -> Result<ApiKeyInfo, String> {
    let info = config::save_api_key(&app_handle, &api_key)?;
    reset_session(true);
    Ok(info)
}

#[tauri::command]
pub async fn immich_test_connection(
    server_url: String,
    api_key: String,
) -> Result<ConnectionInfo, String> {
    let client = ImmichClient::new(&server_url, &api_key)?;
    let version = client.version_numbers().await?;
    let version_text = format!("{}.{}.{}", version.0, version.1, version.2);
    if version < MIN_VERSION {
        return Err(format!(
            "Immich {version_text} is too old. RapidRAW needs Immich {}.{}.{} or newer.",
            MIN_VERSION.0, MIN_VERSION.1, MIN_VERSION.2
        ));
    }
    let user = client.me().await?;
    Ok(ConnectionInfo {
        version: version_text,
        user_name: user.name,
        user_email: user.email,
    })
}

#[tauri::command]
pub async fn immich_list_albums(app_handle: AppHandle) -> Result<Vec<Album>, String> {
    let session = session(&app_handle)?;
    let mut albums = session.client.albums().await?;
    albums.sort_by_key(|a| a.album_name.to_lowercase());
    Ok(albums)
}

#[tauri::command]
pub async fn immich_timeline(app_handle: AppHandle) -> Result<Vec<TimelineMonth>, String> {
    session(&app_handle)?.client.timeline_months().await
}

#[tauri::command]
pub fn immich_prepare_export(
    album: String,
    with_original: bool,
    replace_original: Option<bool>,
    app_handle: AppHandle,
) -> Result<String, String> {
    super::uploads::prepare_export(
        &app_handle,
        &album,
        with_original,
        replace_original.unwrap_or(true),
    )
    .map(|folder| folder.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn immich_remove_from_album(
    album_id: String,
    paths: Vec<String>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let session = session(&app_handle)?;
    let ids: Vec<String> = paths
        .iter()
        .filter(|path| !path.contains("?vc="))
        .filter_map(|path| {
            registry::get(&crate::file_management::parse_virtual_path(path).0)
                .map(|entry| entry.listed_asset_id)
        })
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    session.client.remove_from_album(&album_id, &ids).await
}

#[tauri::command]
pub async fn immich_flush(app_handle: AppHandle) -> sync::SyncStatus {
    if let Some(session) = super::current_session() {
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            sync::send_unsent(&app_handle, &session),
        )
        .await;
    }
    sync::status()
}

#[tauri::command]
pub fn immich_sync_status() -> sync::SyncStatus {
    sync::status()
}

/// What the library shows about the Immich stack of an image.
#[derive(Serialize, Clone, Debug)]
pub struct StackBadge {
    pub stack_id: String,
    /// Files in the stack that can be opened.
    pub count: usize,
    /// This file is the cover (primary asset) of the stack.
    pub is_primary: bool,
    /// For a file shown by expanding a stack: the library entry of the stack.
    pub parent: Option<String>,
}

#[derive(Serialize)]
pub struct ImmichImageFile {
    #[serde(flatten)]
    file: ImageFile,
    #[serde(skip_serializing_if = "Option::is_none")]
    immich_stack: Option<StackBadge>,
    /// Immich favorite (for a stack entry: any of its files).
    immich_favorite: bool,
}

/// The library entry (without the virtual copy suffix) of a listed path.
fn entry_path(path: &str) -> PathBuf {
    crate::file_management::parse_virtual_path(path).0
}

fn favorite_of(path: &str) -> bool {
    registry::is_favorite(&entry_path(path))
}

fn member_image(member: &registry::StackMember, album_id: Option<String>) -> registry::RemoteImage {
    registry::RemoteImage {
        asset_id: member.asset_id.clone(),
        listed_asset_id: member.asset_id.clone(),
        album_id,
    }
}

#[tauri::command]
pub async fn immich_get_images(
    filter: resolve::Filter,
    app_handle: AppHandle,
) -> Result<Vec<ImmichImageFile>, String> {
    let session = session(&app_handle)?;
    let resolved = resolve::listing(&session.client, &filter, &session.cache_dir).await?;

    let mut local_paths = Vec::new();
    let mut placeholders = Vec::new();
    let mut badges = HashMap::new();
    let mut favorites = HashMap::new();
    for item in resolved {
        let path = item.path.to_string_lossy().into_owned();
        favorites.insert(item.path.clone(), item.is_favorite);
        if item.path.exists() {
            local_paths.extend(virtual_copies(&item.path));
            local_paths.push(path.clone());
        } else {
            placeholders.push(placeholder_file(&path, item.file_modified_at.as_deref()));
        }
        registry::set_favorite(&item.image.listed_asset_id, item.listed_favorite);
        if let Some(stack) = item.stack {
            for member in &stack.members {
                registry::set_favorite(&member.asset_id, member.is_favorite);
                registry::insert_member(
                    member.path.clone(),
                    member_image(member, stack.album_id.clone()),
                );
            }
            badges.insert(
                path,
                StackBadge {
                    stack_id: stack.stack_id.clone(),
                    count: stack.members.len(),
                    is_primary: stack
                        .members
                        .iter()
                        .any(|m| m.is_primary && m.asset_id == item.image.asset_id),
                    parent: None,
                },
            );
            registry::insert_stack(item.path.clone(), stack);
        }
        registry::insert(item.path, item.image);
    }

    let mut files =
        crate::file_management::get_album_images(local_paths, app_handle.clone()).await?;
    files.extend(placeholders);
    Ok(files
        .into_iter()
        .map(|file| ImmichImageFile {
            immich_stack: badges.get(&file.path).cloned(),
            immich_favorite: favorites
                .get(&entry_path(&file.path))
                .copied()
                .unwrap_or_else(|| favorite_of(&file.path)),
            file,
        })
        .collect())
}

/// Number of images marked as favorites in Immich.
#[tauri::command]
pub async fn immich_favorites_count(app_handle: AppHandle) -> Result<u64, String> {
    session(&app_handle)?
        .client
        .count(json!({ "isFavorite": true, "type": "IMAGE" }))
        .await
}

/// Marks (or unmarks) the given library entries as Immich favorites, and
/// returns the favorite state of the `refresh` entries afterwards (the
/// entries and the other files of their stacks, which may change too).
#[tauri::command]
pub async fn immich_set_favorite(
    paths: Vec<String>,
    favorite: bool,
    refresh: Vec<String>,
    app_handle: AppHandle,
) -> Result<HashMap<String, bool>, String> {
    let session = session(&app_handle)?;
    let mut ids: Vec<String> = paths
        .iter()
        .flat_map(|path| registry::favorite_targets(&entry_path(path), favorite))
        .collect();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return Err("These images are not in Immich.".to_string());
    }
    session.client.set_favorite(&ids, favorite).await?;
    for id in &ids {
        registry::set_favorite(id, favorite);
    }
    Ok(paths
        .iter()
        .chain(refresh.iter())
        .map(|path| (path.clone(), favorite_of(path)))
        .collect())
}

/// The other files of the Immich stack shown as `path` in the library,
/// cover first, ready to be opened like any listed image.
#[tauri::command]
pub async fn immich_stack_members(
    path: String,
    app_handle: AppHandle,
) -> Result<Vec<ImmichImageFile>, String> {
    let entry_path = PathBuf::from(&path);
    let stack = registry::stack(&entry_path).ok_or("This image is not an Immich stack.")?;

    let mut local_paths = Vec::new();
    let mut placeholders = Vec::new();
    let mut order = HashMap::new();
    for member in stack.members.iter().filter(|m| m.path != entry_path) {
        registry::insert_member(
            member.path.clone(),
            member_image(member, stack.album_id.clone()),
        );
        let member_path = member.path.to_string_lossy().into_owned();
        order.insert(member_path.clone(), (order.len(), member.is_primary));
        if member.path.exists() {
            local_paths.push(member_path);
        } else {
            placeholders.push(placeholder_file(
                &member_path,
                member.file_modified_at.as_deref(),
            ));
        }
    }

    let mut files =
        crate::file_management::get_album_images(local_paths, app_handle.clone()).await?;
    files.extend(placeholders);
    let mut files: Vec<_> = files
        .into_iter()
        .filter_map(|file| {
            let (position, is_primary) = *order.get(&file.path)?;
            Some((
                position,
                ImmichImageFile {
                    immich_favorite: favorite_of(&file.path),
                    immich_stack: Some(StackBadge {
                        stack_id: stack.stack_id.clone(),
                        count: stack.members.len(),
                        is_primary,
                        parent: Some(path.clone()),
                    }),
                    file,
                },
            ))
        })
        .collect();
    files.sort_by_key(|(position, _)| *position);
    Ok(files.into_iter().map(|(_, file)| file).collect())
}

fn placeholder_file(path: &str, file_modified_at: Option<&str>) -> ImageFile {
    let modified = file_modified_at
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.timestamp().max(0) as u64)
        .unwrap_or(0);
    ImageFile::placeholder(path.to_string(), modified)
}

fn virtual_copies(path: &Path) -> Vec<String> {
    let (Some(folder), Some(name)) = (path.parent(), path.file_name()) else {
        return Vec::new();
    };
    let name = name.to_string_lossy();
    let Ok(files) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    files
        .flatten()
        .filter_map(|file| {
            let file_name = file.file_name().to_string_lossy().into_owned();
            match crate::file_management::parse_sidecar_name(&file_name) {
                Some((base, Some(id))) if base == name => {
                    Some(format!("{}?vc={id}", path.display()))
                }
                _ => None,
            }
        })
        .collect()
}
