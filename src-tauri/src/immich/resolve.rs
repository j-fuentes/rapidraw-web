use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::client::{Asset, ImmichClient, Stack};
use super::registry::{self, RemoteImage, StackEntry, StackMember};

#[derive(Debug, Clone)]
pub struct Resolved {
    pub path: PathBuf,
    pub image: RemoteImage,
    pub file_modified_at: Option<String>,
    /// The Immich stack this entry stands for (two or more files).
    pub stack: Option<StackEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Filter {
    pub album_id: Option<String>,
    pub taken_from: Option<String>,
    pub taken_until: Option<String>,
}

impl Filter {
    fn to_query(&self) -> Value {
        let mut query = json!({ "type": "IMAGE" });
        if let Some(album) = &self.album_id {
            query["albumIds"] = json!([album]);
        }
        if let Some(from) = day_start(self.taken_from.as_deref(), 0) {
            query["takenAfter"] = json!(from);
        }
        if let Some(until) = day_start(self.taken_until.as_deref(), 1) {
            query["takenBefore"] = json!(until);
        }
        query
    }
}

fn day_start(day: Option<&str>, offset: i64) -> Option<String> {
    let date = chrono::NaiveDate::parse_from_str(day?.trim(), "%Y-%m-%d").ok()?;
    let date = date.checked_add_signed(chrono::Duration::days(offset))?;
    Some(format!("{}T00:00:00.000Z", date.format("%Y-%m-%d")))
}

fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim_start_matches('.');
    if trimmed.is_empty() {
        "image".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn cache_path(asset: &Asset, cache_dir: &Path) -> PathBuf {
    cache_dir
        .join(&asset.id)
        .join(safe_file_name(&asset.original_file_name))
}

struct StackIndex {
    stacks: Vec<Stack>,
    by_asset: HashMap<String, usize>,
}

impl StackIndex {
    fn new(stacks: Vec<Stack>) -> Self {
        let by_asset = stacks
            .iter()
            .enumerate()
            .flat_map(|(i, s)| s.assets.iter().map(move |a| (a.id.clone(), i)))
            .collect();
        Self { stacks, by_asset }
    }

    fn index_of(&self, asset: &Asset) -> Option<usize> {
        self.by_asset.get(&asset.id).copied()
    }

    /// The files of a stack that can be opened (no trashed or unsupported
    /// ones), cover first.
    fn members(&self, index: usize, album_id: Option<&str>, cache_dir: &Path) -> StackEntry {
        let stack = &self.stacks[index];
        let mut seen = HashSet::new();
        let mut members: Vec<StackMember> = stack
            .assets
            .iter()
            .filter(|a| !a.is_trashed && a.kind == "IMAGE")
            .filter_map(|a| {
                let path = cache_path(a, cache_dir);
                (crate::formats::is_supported_image_file(&path) && seen.insert(path.clone())).then(
                    || StackMember {
                        path,
                        asset_id: a.id.clone(),
                        file_modified_at: a.file_modified_at.clone(),
                        is_primary: a.id == stack.primary_asset_id,
                    },
                )
            })
            .collect();
        members.sort_by(|a, b| {
            b.is_primary.cmp(&a.is_primary).then_with(|| {
                let name = |m: &StackMember| m.path.file_name().map(|n| n.to_ascii_lowercase());
                name(a).cmp(&name(b))
            })
        });
        StackEntry {
            stack_id: stack.id.clone(),
            album_id: album_id.map(str::to_string),
            members,
        }
    }

    fn source<'a>(&'a self, listed: &'a Asset, cache_dir: &Path) -> &'a Asset {
        if is_raw(listed) {
            return listed;
        }
        self.by_asset
            .get(&listed.id)
            .map(|&i| self.stacks[i].assets.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|a| a.id != listed.id && !a.is_trashed && is_raw(a))
            .max_by_key(|raw| (cache_path(raw, cache_dir).exists(), same_stem(raw, listed)))
            .unwrap_or(listed)
    }
}

fn same_stem(a: &Asset, b: &Asset) -> bool {
    let stem = |asset: &Asset| {
        Path::new(&asset.original_file_name)
            .file_stem()
            .map(|s| s.to_ascii_lowercase())
    };
    stem(a) == stem(b)
}

fn is_raw(asset: &Asset) -> bool {
    crate::formats::is_raw_file(&asset.original_file_name)
}

fn month(day: Option<&str>) -> Option<&str> {
    day.map(str::trim).and_then(|day| day.get(..7))
}

/// The stacks that contain any of the listed assets, without asking Immich
/// for every stack of the library (`GET /stacks` is huge on big libraries).
///
/// The timeline with collapsed stacks gives, for the album or the months of
/// the filter, which assets are unstacked and the stack id of every primary
/// asset. Only those stacks are fetched. Listed assets the timeline did not
/// show and that are not in one of those stacks (non-primary assets whose
/// primary is not listed, assets near a month border) are looked up one by
/// one.
async fn stacks_for(
    client: &ImmichClient,
    filter: &Filter,
    assets: &[Asset],
) -> Result<Vec<Stack>, String> {
    let timeline = client
        .timeline_stacks(
            filter.album_id.as_deref(),
            month(filter.taken_from.as_deref()),
            month(filter.taken_until.as_deref()),
        )
        .await
        .unwrap_or_else(|e| {
            log::warn!("Immich timeline unavailable, looking up stacks per asset: {e}");
            HashMap::new()
        });

    let mut stack_ids: Vec<String> = assets
        .iter()
        .filter_map(|a| timeline.get(&a.id).cloned().flatten())
        .collect();
    stack_ids.sort();
    stack_ids.dedup();
    let mut stacks = client.stacks_by_id(stack_ids.iter().cloned()).await?;

    let known: HashSet<&str> = timeline
        .keys()
        .map(String::as_str)
        .chain(
            stacks
                .iter()
                .flat_map(|s| s.assets.iter().map(|a| a.id.as_str())),
        )
        .collect();
    let unknown: Vec<String> = assets
        .iter()
        .filter(|a| !known.contains(a.id.as_str()))
        .map(|a| a.id.clone())
        .collect();
    if !unknown.is_empty() {
        let more: Vec<String> = client
            .stack_ids_of(&unknown)
            .await?
            .into_iter()
            .filter(|id| !stack_ids.contains(id))
            .collect();
        stacks.extend(client.stacks_by_id(more).await?);
    }
    Ok(stacks)
}

async fn listed_assets(client: &ImmichClient, filter: &Filter) -> Result<Vec<Asset>, String> {
    let mut query = filter.to_query();
    query["order"] = json!("desc");
    Ok(client
        .search(query)
        .await?
        .into_iter()
        .filter(|asset| asset.kind == "IMAGE" && !registry::is_trashing(&asset.id))
        .collect())
}

pub async fn listing(
    client: &ImmichClient,
    filter: &Filter,
    cache_dir: &Path,
) -> Result<Vec<Resolved>, String> {
    let assets = listed_assets(client, filter).await?;
    let stacks = StackIndex::new(stacks_for(client, filter, &assets).await?);
    Ok(resolve(&assets, &stacks, filter, cache_dir))
}

fn resolve(
    assets: &[Asset],
    stacks: &StackIndex,
    filter: &Filter,
    cache_dir: &Path,
) -> Vec<Resolved> {
    // One entry per stack: its primary asset when listed, otherwise the
    // first listed asset of the stack.
    let listed_ids: HashSet<&str> = assets.iter().map(|a| a.id.as_str()).collect();
    let mut shown_for_stack: HashMap<usize, &str> = HashMap::new();
    for listed in assets {
        if let Some(index) = stacks.index_of(listed) {
            let primary = stacks.stacks[index].primary_asset_id.as_str();
            let shown = if listed_ids.contains(primary) {
                primary
            } else {
                listed.id.as_str()
            };
            shown_for_stack.entry(index).or_insert(shown);
        }
    }

    let mut seen = HashSet::new();
    let mut resolved = Vec::with_capacity(assets.len());
    for listed in assets {
        let index = stacks.index_of(listed);
        if index.is_some_and(|i| shown_for_stack.get(&i) != Some(&listed.id.as_str())) {
            continue;
        }
        let source = stacks.source(listed, cache_dir);
        let path = cache_path(source, cache_dir);
        if !crate::formats::is_supported_image_file(&path) || !seen.insert(path.clone()) {
            continue;
        }
        let stack = index
            .map(|i| stacks.members(i, filter.album_id.as_deref(), cache_dir))
            .filter(|stack| stack.members.len() > 1);
        resolved.push(Resolved {
            path,
            file_modified_at: source.file_modified_at.clone(),
            image: RemoteImage {
                asset_id: source.id.clone(),
                listed_asset_id: listed.id.clone(),
                album_id: filter.album_id.clone(),
            },
            stack,
        });
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn summary(resolved: &[Resolved]) -> Vec<(String, String, String)> {
        let mut rows: Vec<_> = resolved
            .iter()
            .map(|r| {
                (
                    r.image.listed_asset_id.clone(),
                    r.image.asset_id.clone(),
                    r.path.to_string_lossy().into_owned(),
                )
            })
            .collect();
        rows.sort();
        rows
    }

    fn asset(n: u8, name: &str) -> Asset {
        serde_json::from_value(json!({
            "id": format!("00000000-0000-4000-8000-0000000000{n:02x}"),
            "type": "IMAGE",
            "originalFileName": name,
        }))
        .unwrap()
    }

    fn stack(n: u8, primary: &Asset, assets: &[&Asset]) -> Stack {
        Stack {
            id: format!("stack-{n}"),
            primary_asset_id: primary.id.clone(),
            assets: assets.iter().map(|a| (*a).clone()).collect(),
        }
    }

    fn names(entry: &Resolved) -> Vec<String> {
        entry
            .stack
            .as_ref()
            .map(|s| {
                s.members
                    .iter()
                    .map(|m| {
                        let name = m.path.file_name().unwrap().to_string_lossy().into_owned();
                        if m.is_primary {
                            format!("*{name}")
                        } else {
                            name
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn one_entry_per_stack_with_its_members() {
        let cache = Path::new("/cache");
        let jpg = asset(1, "DSCF0001.JPG");
        let raf = asset(2, "DSCF0001.RAF");
        let other_raf = asset(3, "DSCF0000.RAF");
        let burst_cover = asset(4, "DSCF6437.RAF");
        let burst_a = asset(5, "DSCF6438.RAF");
        let burst_b = asset(6, "DSCF6439.RAF");
        let single = asset(7, "IMG_1.HEIC");
        let mut trashed = asset(8, "IMG_2.JPG");
        trashed.is_trashed = true;
        let lonely = asset(9, "IMG_2.HEIC");

        let stacks = StackIndex::new(vec![
            stack(1, &jpg, &[&jpg, &other_raf, &raf]),
            stack(2, &burst_cover, &[&burst_cover, &burst_b, &burst_a]),
            stack(3, &lonely, &[&lonely, &trashed]),
        ]);
        // The burst cover is not in the album; the RAWs of the pair are not
        // listed either (as in an album with only the JPG).
        let listed = vec![
            jpg.clone(),
            burst_b.clone(),
            burst_a.clone(),
            single.clone(),
            lonely.clone(),
        ];
        let resolved = resolve(&listed, &stacks, &Filter::default(), cache);
        assert_eq!(resolved.len(), 4);

        // RAW+JPG: opens the RAW with the same name, lists all the files.
        assert_eq!(resolved[0].path, cache_path(&raf, cache));
        assert_eq!(resolved[0].image.listed_asset_id, jpg.id);
        assert_eq!(
            names(&resolved[0]),
            ["*DSCF0001.JPG", "DSCF0000.RAF", "DSCF0001.RAF"]
        );

        // Burst whose cover is not listed: a single entry, the first listed.
        assert_eq!(resolved[1].image.asset_id, burst_b.id);
        assert_eq!(
            names(&resolved[1]),
            ["*DSCF6437.RAF", "DSCF6438.RAF", "DSCF6439.RAF"]
        );

        // Not stacked, and a stack left with one usable file: no stack.
        assert_eq!(resolved[2].image.asset_id, single.id);
        assert!(resolved[2].stack.is_none());
        assert_eq!(resolved[3].image.asset_id, lonely.id);
        assert!(resolved[3].stack.is_none());
    }

    /// Same listing as with the full `GET /stacks`, for an album
    /// (IMMICH_TEST_ALBUM) and a month (IMMICH_TEST_MONTH, "YYYY-MM").
    #[test]
    #[ignore = "needs IMMICH_TEST_URL and IMMICH_API_KEY"]
    fn live_listing_matches_full_stack_list() {
        let url = std::env::var("IMMICH_TEST_URL").expect("IMMICH_TEST_URL");
        let key = std::env::var("IMMICH_API_KEY").expect("IMMICH_API_KEY");
        let client = ImmichClient::new(&url, &key).unwrap();
        let cache_dir = std::env::temp_dir().join("rapidraw-immich-resolve-test");

        let mut filters = Vec::new();
        if let Ok(album) = std::env::var("IMMICH_TEST_ALBUM") {
            filters.push(Filter {
                album_id: Some(album),
                ..Default::default()
            });
        }
        if let Ok(month) = std::env::var("IMMICH_TEST_MONTH") {
            let first =
                chrono::NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").unwrap();
            let last = first
                .checked_add_months(chrono::Months::new(1))
                .unwrap()
                .pred_opt()
                .unwrap();
            filters.push(Filter {
                taken_from: Some(first.to_string()),
                taken_until: Some(last.to_string()),
                ..Default::default()
            });
        }
        assert!(
            !filters.is_empty(),
            "set IMMICH_TEST_ALBUM and/or IMMICH_TEST_MONTH"
        );

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let started = Instant::now();
            let all = StackIndex::new(client.all_stacks().await.unwrap());
            println!(
                "full GET /stacks: {} stacks in {:?}",
                all.stacks.len(),
                started.elapsed()
            );
            for filter in &filters {
                let assets = listed_assets(&client, filter).await.unwrap();
                let started = Instant::now();
                let stacks = stacks_for(&client, filter, &assets).await.unwrap();
                let took = started.elapsed();
                let count = stacks.len();
                let new = resolve(&assets, &StackIndex::new(stacks), filter, &cache_dir);
                let old = resolve(&assets, &all, filter, &cache_dir);
                println!(
                    "{filter:?}: {} assets, {} listed, {count} stacks fetched in {took:?}",
                    assets.len(),
                    new.len()
                );
                assert_eq!(summary(&new), summary(&old));

                // IMMICH_TEST_SHOW=DSCF64,IMG_ prints the stacks of those files.
                let show = std::env::var("IMMICH_TEST_SHOW").unwrap_or_default();
                let wanted: Vec<&str> = show.split(',').filter(|s| !s.is_empty()).collect();
                let stacked = new.iter().filter(|r| r.stack.is_some()).count();
                println!("  {stacked} entries are stacks");
                for entry in &new {
                    let members = names(entry);
                    if members.iter().any(|m| wanted.iter().any(|w| m.contains(w))) {
                        let name = entry.path.file_name().unwrap().to_string_lossy();
                        println!("  {name}: {members:?}");
                    }
                }
            }
        });
    }
}
