use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct RemoteImage {
    pub asset_id: String,
    pub listed_asset_id: String,
    pub album_id: Option<String>,
}

static ENTRIES: Lazy<RwLock<HashMap<PathBuf, RemoteImage>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// One file of an Immich stack, as it can be opened from the library.
#[derive(Debug, Clone)]
pub struct StackMember {
    pub path: PathBuf,
    pub asset_id: String,
    pub file_modified_at: Option<String>,
    pub is_primary: bool,
    pub is_favorite: bool,
}

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub stack_id: String,
    pub album_id: Option<String>,
    pub members: Vec<StackMember>,
}

/// Stacks by the path of the library entry that stands for them.
static STACKS: Lazy<RwLock<HashMap<PathBuf, StackEntry>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn insert_stack(path: PathBuf, stack: StackEntry) {
    STACKS.write().unwrap().insert(path, stack);
}

pub fn stack(path: &Path) -> Option<StackEntry> {
    STACKS.read().unwrap().get(path).cloned()
}

/// Registers a stack member so it can be opened, without replacing an
/// entry the listing already made for that file.
pub fn insert_member(path: PathBuf, image: RemoteImage) {
    ENTRIES.write().unwrap().entry(path).or_insert(image);
}

pub fn insert(path: PathBuf, image: RemoteImage) {
    ENTRIES.write().unwrap().insert(path, image);
}

pub fn get(path: &Path) -> Option<RemoteImage> {
    ENTRIES.read().unwrap().get(path).cloned()
}

pub fn set_listed_asset(path: &Path, listed_asset_id: &str) {
    if let Some(entry) = ENTRIES.write().unwrap().get_mut(path) {
        entry.listed_asset_id = listed_asset_id.to_string();
    }
}

pub fn is_placeholder(path: &Path) -> bool {
    let known = ENTRIES
        .read()
        .map(|entries| entries.contains_key(path))
        .unwrap_or(false);
    known && !path.exists()
}

pub fn remove(path: &Path) {
    ENTRIES.write().unwrap().remove(path);
}

static TRASHING: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));

pub fn mark_trashing(asset_ids: &[String]) {
    TRASHING.write().unwrap().extend(asset_ids.iter().cloned());
}

pub fn done_trashing(asset_ids: &[String]) {
    let mut trashing = TRASHING.write().unwrap();
    for id in asset_ids {
        trashing.remove(id);
    }
}

pub fn is_trashing(asset_id: &str) -> bool {
    TRASHING.read().unwrap().contains(asset_id)
}

/// Favorite state of the assets seen in listings, by asset id.
static FAVORITES: Lazy<RwLock<HashMap<String, bool>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub fn set_favorite(asset_id: &str, favorite: bool) {
    FAVORITES
        .write()
        .unwrap()
        .insert(asset_id.to_string(), favorite);
}

fn is_favorite_asset(asset_id: &str) -> bool {
    FAVORITES
        .read()
        .unwrap()
        .get(asset_id)
        .copied()
        .unwrap_or(false)
}

/// Whether the library entry at `path` is an Immich favorite: its listed
/// asset, or for an entry that stands for a stack, any file of the stack.
pub fn is_favorite(path: &Path) -> bool {
    if let Some(stack) = stack(path) {
        if stack.members.iter().any(|m| is_favorite_asset(&m.asset_id)) {
            return true;
        }
    }
    get(path).is_some_and(|entry| is_favorite_asset(&entry.listed_asset_id))
}

/// The assets to change so that the entry at `path` becomes (or stops
/// being) a favorite: marking it marks the asset Immich shows for it (the
/// listed one), unmarking it unmarks every favorite file of its stack.
pub fn favorite_targets(path: &Path, favorite: bool) -> Vec<String> {
    let Some(entry) = get(path) else {
        return Vec::new();
    };
    if favorite {
        return vec![entry.listed_asset_id];
    }
    let mut ids = vec![entry.listed_asset_id];
    if let Some(stack) = stack(path) {
        ids.extend(
            stack
                .members
                .iter()
                .filter(|m| is_favorite_asset(&m.asset_id))
                .map(|m| m.asset_id.clone()),
        );
    }
    ids.sort();
    ids.dedup();
    ids
}

pub fn clear() {
    ENTRIES.write().unwrap().clear();
    STACKS.write().unwrap().clear();
    FAVORITES.write().unwrap().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(id: &str) -> RemoteImage {
        RemoteImage {
            asset_id: id.to_string(),
            listed_asset_id: id.to_string(),
            album_id: None,
        }
    }

    fn member(path: &str, id: &str, is_primary: bool) -> StackMember {
        StackMember {
            path: PathBuf::from(path),
            asset_id: id.to_string(),
            file_modified_at: None,
            is_primary,
            is_favorite: false,
        }
    }

    #[test]
    fn favorites_of_entries_and_stacks() {
        // Unique paths and ids: the registry is shared by the tests.
        let entry = PathBuf::from("/fav-test/cover/IMG_1.heic");
        let burst = PathBuf::from("/fav-test/burst/IMG_2.heic");
        let single = PathBuf::from("/fav-test/single/IMG_3.jpg");
        insert(entry.clone(), image("fav-cover"));
        insert(single.clone(), image("fav-single"));
        insert_member(burst.clone(), image("fav-burst"));
        insert_stack(
            entry.clone(),
            StackEntry {
                stack_id: "fav-stack".to_string(),
                album_id: None,
                members: vec![
                    member("/fav-test/cover/IMG_1.heic", "fav-cover", true),
                    member("/fav-test/burst/IMG_2.heic", "fav-burst", false),
                ],
            },
        );
        for id in ["fav-cover", "fav-burst", "fav-single"] {
            set_favorite(id, false);
        }
        assert!(!is_favorite(&entry));
        assert!(!is_favorite(&single));
        assert!(!is_favorite(Path::new("/fav-test/unknown.jpg")));

        // A favorite file inside the stack makes the entry a favorite; the
        // file itself (when the stack is expanded) too, the cover not.
        set_favorite("fav-burst", true);
        assert!(is_favorite(&entry));
        assert!(is_favorite(&burst));

        // Marking the entry marks the asset Immich shows (the cover);
        // unmarking it unmarks every favorite file of the stack.
        assert_eq!(favorite_targets(&entry, true), ["fav-cover"]);
        assert_eq!(favorite_targets(&entry, false), ["fav-burst", "fav-cover"]);
        assert_eq!(favorite_targets(&burst, true), ["fav-burst"]);
        assert_eq!(favorite_targets(&single, false), ["fav-single"]);
        assert!(favorite_targets(Path::new("/fav-test/unknown.jpg"), true).is_empty());
    }
}
