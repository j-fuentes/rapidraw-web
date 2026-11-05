use once_cell::sync::Lazy;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use super::{ImmichClient, Session, registry, session, sync};

#[derive(Debug, Clone)]
struct ExportTarget {
    album: ExportAlbum,
    with_original: bool,
    /// For images that come from Immich: the export takes the original's place in its albums and as a favorite
    /// (setting "Replace the original in albums and favorites")
    replace_original: bool,
}

#[derive(Debug, Clone)]
enum ExportAlbum {
    None,
    Source,
    Album(String),
}

static EXPORT_TARGETS: Lazy<Mutex<HashMap<PathBuf, ExportTarget>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn prepare_export(
    app_handle: &AppHandle,
    album: &str,
    with_original: bool,
    replace_original: bool,
) -> Result<PathBuf, String> {
    session(app_handle)?;
    let root = app_handle
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("immich-exports");
    let mut targets = EXPORT_TARGETS.lock().unwrap();
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            if !targets.contains_key(&entry.path()) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    let folder = root.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let album = match album {
        "" | "none" => ExportAlbum::None,
        "source" => ExportAlbum::Source,
        id => ExportAlbum::Album(id.to_string()),
    };
    targets.insert(
        folder.clone(),
        ExportTarget {
            album,
            with_original,
            replace_original,
        },
    );
    Ok(folder)
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn is_export_path(path: &Path) -> bool {
    let targets = EXPORT_TARGETS.lock().unwrap();
    path.ancestors()
        .skip(1)
        .any(|folder| targets.contains_key(folder))
}

pub fn on_exported(
    app_handle: &AppHandle,
    source_path: &str,
    output_path: &Path,
) -> Result<(), String> {
    let Some(target) = output_path
        .ancestors()
        .skip(1)
        .find_map(|folder| EXPORT_TARGETS.lock().unwrap().get(folder).cloned())
    else {
        return Ok(());
    };
    let session = session(app_handle)?;

    let (source, _) = crate::file_management::parse_virtual_path(source_path);
    let file_name = output_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let result = tauri::async_runtime::block_on(upload_export(
        app_handle,
        &session,
        &source,
        &target,
        output_path,
    ));
    let _ = std::fs::remove_file(output_path);
    if result.is_ok() {
        let _ = app_handle.emit("immich-upload", json!({ "fileName": file_name }));
    }
    let _ = app_handle.emit("immich-library-changed", json!({}));
    result.map_err(|e| format!("Immich upload of {file_name} failed: {e}"))
}

async fn upload_export(
    app_handle: &AppHandle,
    session: &Session,
    source: &Path,
    target: &ExportTarget,
    output: &Path,
) -> Result<(), String> {
    let client = &session.client;
    let entry = registry::get(source);
    let original = match &entry {
        Some(entry) => {
            if let Err(e) = sync::push(app_handle, client, &entry.asset_id, source).await {
                log::warn!("Could not send the edits of {}: {e}", source.display());
            }
            Some(entry.asset_id.clone())
        }
        None if target.with_original => Some(upload_original(app_handle, session, source).await?),
        None => None,
    };

    let album = match &target.album {
        ExportAlbum::None => None,
        ExportAlbum::Source => entry.as_ref().and_then(|e| e.album_id.clone()),
        ExportAlbum::Album(id) => Some(id.clone()),
    };
    // only for images that come from Immich (not for an original uploaded from a local folder just now)
    let replace = target.replace_original && entry.is_some();
    let Some(new_id) = upload_stacked(client, output, original, album, replace).await? else {
        return Ok(());
    };
    if entry.is_some() {
        registry::set_listed_asset(source, &new_id);
    }
    Ok(())
}

async fn upload_stacked(
    client: &ImmichClient,
    output: &Path,
    original: Option<String>,
    album: Option<String>,
    replace: bool,
) -> Result<Option<String>, String> {
    let modified = file_time(output);
    let new_id = client.upload(output, &modified, &modified).await?.id;
    if original.as_deref() == Some(new_id.as_str()) {
        return Ok(None);
    }

    if let Some(album) = album {
        client
            .add_to_album(&album, std::slice::from_ref(&new_id))
            .await?;
    }
    if let Some(original) = original {
        let mut ids = vec![new_id.clone(), original.clone()];
        let members = client
            .stack_of(&original)
            .await?
            .map(|stack| stack.assets)
            .unwrap_or_default();
        for asset in &members {
            if !ids.contains(&asset.id) {
                ids.push(asset.id.clone());
            }
        }
        client.create_stack(&ids).await?;
        if replace {
            // the export is already uploaded and stacked: a failure here is reported in the log, not as a
            // failed export
            match replace_original(client, &new_id, &original, &members).await {
                Ok(report) => log::info!("Immich export {new_id}: {report}"),
                Err(e) => log::warn!(
                    "Immich export {new_id}: could not replace the original in albums/favorites: {e}"
                ),
            }
        }
    }
    Ok(Some(new_id))
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct ReplaceReport {
    pub favorite: bool,
    /// album id → stack members taken out of it
    pub albums: Vec<(String, Vec<String>)>,
    pub errors: Vec<String>,
}

impl std::fmt::Display for ReplaceReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "favorite={}, replaced in {} album(s){}",
            self.favorite,
            self.albums.len(),
            if self.errors.is_empty() {
                String::new()
            } else {
                format!(", errors: {}", self.errors.join("; "))
            }
        )
    }
}

/// What replacing the original means for each album: the stack members (other than the export) that are in it.
/// Albums that only contain the export already are left alone.
pub(crate) fn albums_to_replace(
    in_albums: &[(String, Vec<String>)],
    new_id: &str,
) -> Vec<(String, Vec<String>)> {
    let mut by_album: Vec<(String, Vec<String>)> = Vec::new();
    for (asset, albums) in in_albums {
        if asset == new_id {
            continue;
        }
        for album in albums {
            match by_album.iter_mut().find(|(id, _)| id == album) {
                Some((_, assets)) => {
                    if !assets.contains(asset) {
                        assets.push(asset.clone())
                    }
                }
                None => by_album.push((album.clone(), vec![asset.clone()])),
            }
        }
    }
    by_album
}

/// The export takes the place of the original (and of the rest of its stack) in albums and favorites:
/// (a) favorite if the original or any member of its stack is one; (b) in every album with the original or another
/// stack member, the export is added and those members are taken out. Nothing is deleted or unstacked: only which
/// assets are in which album, and the export's favorite flag, change.
pub(crate) async fn replace_original(
    client: &ImmichClient,
    new_id: &str,
    original: &str,
    members: &[crate::immich::client::Asset],
) -> Result<ReplaceReport, String> {
    let mut report = ReplaceReport::default();
    let mut others: Vec<String> = members
        .iter()
        .map(|a| a.id.clone())
        .filter(|id| id != new_id)
        .collect();
    if !others.iter().any(|id| id == original) {
        others.insert(0, original.to_string());
    }

    // (a) favorites; an original without a stack is not in members, so it is asked for
    let mut favorite = members.iter().any(|a| a.id != new_id && a.is_favorite);
    if !favorite && !members.iter().any(|a| a.id == original) {
        favorite = client.asset(original).await?.is_favorite;
    }
    if favorite {
        client.set_favorite(&[new_id.to_string()], true).await?;
        report.favorite = true;
    }

    // (b) albums
    let mut in_albums = Vec::new();
    for id in &others {
        let albums = client.albums_of(id).await?;
        in_albums.push((
            id.clone(),
            albums.into_iter().map(|a| a.id).collect::<Vec<_>>(),
        ));
    }
    for (album, assets) in albums_to_replace(&in_albums, new_id) {
        // Added first and checked (add_to_album fails unless the export is really in the album): the stack's other
        // photos only leave an album that has the export, so no stack is ever left without a photo in it. This also
        // covers exporting an "_edited" again: the previous export is a member of the stack and is replaced.
        if let Err(e) = client.add_to_album(&album, &[new_id.to_string()]).await {
            report.errors.push(format!("album {album}: {e}"));
            continue;
        }
        if let Err(e) = client.remove_from_album(&album, &assets).await {
            report.errors.push(format!("album {album}: {e}"));
            continue;
        }
        report.albums.push((album, assets));
    }
    Ok(report)
}

async fn upload_original(
    app_handle: &AppHandle,
    session: &Session,
    source: &Path,
) -> Result<String, String> {
    let modified = file_time(source);
    let id = session
        .client
        .upload(source, &modified, &modified)
        .await?
        .id;

    let file_name = source.file_name().ok_or("Invalid file name")?;
    let target = session.cache_dir.join(&id).join(file_name);
    std::fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::copy(source, &target).map_err(|e| format!("Cannot copy {}: {e}", source.display()))?;
    let sidecar = crate::exif_processing::get_primary_sidecar_path(source);
    if sidecar.exists() {
        let _ = std::fs::copy(
            &sidecar,
            crate::exif_processing::get_primary_sidecar_path(&target),
        );
    }
    registry::insert(
        target.clone(),
        registry::RemoteImage {
            asset_id: id.clone(),
            listed_asset_id: id.clone(),
            album_id: None,
        },
    );
    sync::push(app_handle, &session.client, &id, &target).await?;
    Ok(id)
}

fn file_time(path: &Path) -> String {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(chrono::DateTime::<chrono::Utc>::from)
        .unwrap_or_else(|_| chrono::Utc::now())
        .to_rfc3339()
}

#[cfg(test)]
#[path = "uploads_tests.rs"]
mod tests;
