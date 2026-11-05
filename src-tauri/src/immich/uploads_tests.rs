use super::*;

fn register_export_folder(with_original: bool) -> PathBuf {
    let folder = std::env::temp_dir()
        .join("rapidraw-immich-test")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&folder).unwrap();
    EXPORT_TARGETS.lock().unwrap().insert(
        folder.clone(),
        ExportTarget {
            album: ExportAlbum::None,
            with_original,
            replace_original: true,
        },
    );
    folder
}

fn write_jpeg(path: &Path, seed: u32) {
    let image = image::RgbImage::from_fn(96, 64, |x, y| {
        image::Rgb([
            (seed.wrapping_mul(31).wrapping_add(x * 3)) as u8,
            (seed.wrapping_mul(17).wrapping_add(y * 4)) as u8,
            (seed.wrapping_add(x * y)) as u8,
        ])
    });
    image.save(path).unwrap();
}

#[test]
fn export_paths_are_recognised() {
    let folder = register_export_folder(false);
    assert!(is_export_path(&folder.join("a.jpg")));
    assert!(is_export_path(&folder.join("sub").join("a.jpg")));
    assert!(!is_export_path(&folder));
    assert!(!is_export_path(Path::new("/tmp/not-an-export/a.jpg")));
    EXPORT_TARGETS.lock().unwrap().remove(&folder);
    assert!(!is_export_path(&folder.join("a.jpg")));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
#[ignore = "needs IMMICH_TEST_URL and IMMICH_API_KEY"]
fn live_upload_is_stacked_on_original() {
    let url = std::env::var("IMMICH_TEST_URL").expect("IMMICH_TEST_URL");
    let key = std::env::var("IMMICH_API_KEY").expect("IMMICH_API_KEY");
    let client = ImmichClient::new(&url, &key).unwrap();

    let folder = register_export_folder(true);
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let seed = chrono::Utc::now().timestamp_subsec_nanos();
    let original = folder.join(format!("rapidraw-export-test-{stamp}.jpg"));
    let export = folder.join(format!("rapidraw-export-test-{stamp}_edited.jpg"));
    write_jpeg(&original, seed);
    write_jpeg(&export, seed.wrapping_add(7));
    assert!(is_export_path(&export));

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ids = runtime.block_on(async {
        let modified = file_time(&original);
        let original_id = client
            .upload(&original, &modified, &modified)
            .await
            .unwrap()
            .id;
        let new_id = upload_stacked(&client, &export, Some(original_id.clone()), None, false)
            .await
            .unwrap()
            .expect("export was uploaded as a new asset");
        let stack = client
            .stack_of(&original_id)
            .await
            .unwrap()
            .expect("original is in a stack");
        let listed = client
            .all_stacks()
            .await
            .unwrap()
            .into_iter()
            .find(|stack| stack.assets.iter().any(|asset| asset.id == original_id))
            .expect("original is in the full stack list");
        let ids_of = |stack: &crate::immich::client::Stack| {
            let mut ids: Vec<String> = stack.assets.iter().map(|a| a.id.clone()).collect();
            ids.sort();
            ids
        };
        assert_eq!(listed.primary_asset_id, stack.primary_asset_id);
        assert_eq!(ids_of(&listed), ids_of(&stack));
        assert_eq!(stack.primary_asset_id, new_id);
        assert!(stack.assets.iter().any(|asset| asset.id == new_id));
        vec![original_id, new_id]
    });
    println!("uploaded and stacked: {}", ids.join(","));

    if std::env::var("IMMICH_TEST_KEEP").is_err() {
        runtime.block_on(async {
            let response = reqwest::Client::new()
                .delete(format!("{}/api/assets", url.trim_end_matches('/')))
                .header("x-api-key", key.trim())
                .json(&serde_json::json!({ "ids": ids, "force": true }))
                .send()
                .await
                .unwrap();
            assert!(
                response.status().is_success(),
                "cleanup: {}",
                response.status()
            );
        });
        println!("deleted test assets");
    }
    EXPORT_TARGETS.lock().unwrap().remove(&folder);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn albums_to_replace_groups_members_per_album() {
    let input = vec![
        ("orig".to_string(), vec!["a1".to_string(), "a2".to_string()]),
        ("raw".to_string(), vec!["a2".to_string(), "a3".to_string()]),
        ("new".to_string(), vec!["a1".to_string()]),
        ("solo".to_string(), vec![]),
    ];
    let got = albums_to_replace(&input, "new");
    assert_eq!(
        got,
        vec![
            ("a1".to_string(), vec!["orig".to_string()]),
            (
                "a2".to_string(),
                vec!["orig".to_string(), "raw".to_string()]
            ),
            ("a3".to_string(), vec!["raw".to_string()]),
        ]
    );
    assert!(albums_to_replace(&[], "new").is_empty());
}

/// Live: two test photos stacked (original + a "RAW" that is a favorite), two test albums (one with the original,
/// one with the other member). An export with "replace the original" must end up a favorite, in both albums
/// instead of them, on top of the stack, with no photo deleted or trashed. Everything created is deleted at the end
/// (IMMICH_TEST_KEEP=1 keeps it).
#[test]
#[ignore = "needs IMMICH_TEST_URL and IMMICH_API_KEY"]
fn live_export_replaces_original_in_albums_and_favorites() {
    let url = std::env::var("IMMICH_TEST_URL").expect("IMMICH_TEST_URL");
    let key = std::env::var("IMMICH_API_KEY").expect("IMMICH_API_KEY");
    let client = ImmichClient::new(&url, &key).unwrap();
    let folder = register_export_folder(false);
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let seed = chrono::Utc::now().timestamp_subsec_nanos();
    let original = folder.join(format!("rapidraw-replace-test-{stamp}.jpg"));
    let member = folder.join(format!("rapidraw-replace-test-{stamp}-raw.jpg"));
    let export = folder.join(format!("rapidraw-replace-test-{stamp}_edited.jpg"));
    write_jpeg(&original, seed);
    write_jpeg(&member, seed.wrapping_add(3));
    write_jpeg(&export, seed.wrapping_add(7));

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut assets: Vec<String> = Vec::new();
    let mut albums: Vec<String> = Vec::new();
    let result = runtime.block_on(async {
        let up = |p: PathBuf| {
            let client = &client;
            async move {
                let m = file_time(&p);
                client.upload(&p, &m, &m).await.map(|r| r.id)
            }
        };
        let original_id = up(original.clone()).await?;
        let member_id = up(member.clone()).await?;
        assets.extend([original_id.clone(), member_id.clone()]);
        client
            .create_stack(&[original_id.clone(), member_id.clone()])
            .await?;
        client.set_favorite(&[member_id.clone()], true).await?;
        let a1 = client
            .create_album(
                &format!("zz-rapidraw-replace-test-{stamp}-1"),
                &[original_id.clone()],
            )
            .await?
            .id;
        albums.push(a1.clone());
        let a2 = client
            .create_album(
                &format!("zz-rapidraw-replace-test-{stamp}-2"),
                &[member_id.clone()],
            )
            .await?
            .id;
        albums.push(a2.clone());

        let new_id = upload_stacked(
            &client,
            &export,
            Some(original_id.clone()),
            Some(a1.clone()),
            true,
        )
        .await?
        .expect("export uploaded");
        assets.push(new_id.clone());

        let new = client.asset(&new_id).await?;
        assert!(new.is_favorite, "export marked favorite");
        let in1 = client.album_asset_ids(&a1).await?;
        let in2 = client.album_asset_ids(&a2).await?;
        println!("album1 {in1:?}\nalbum2 {in2:?}");
        assert_eq!(in1, vec![new_id.clone()], "album 1: only the export");
        assert_eq!(in2, vec![new_id.clone()], "album 2: only the export");
        let stack = client.stack_of(&original_id).await?.expect("still stacked");
        assert_eq!(stack.primary_asset_id, new_id);
        let mut ids: Vec<String> = stack.assets.iter().map(|a| a.id.clone()).collect();
        ids.sort();
        let mut want = vec![original_id.clone(), member_id.clone(), new_id.clone()];
        want.sort();
        assert_eq!(ids, want, "stack keeps every photo");
        for id in [&original_id, &member_id] {
            assert!(!client.asset(id).await?.is_trashed, "{id} not trashed");
        }
        assert!(
            client.asset(&member_id).await?.is_favorite,
            "member stays favorite"
        );
        Ok::<_, String>(())
    });

    if std::env::var("IMMICH_TEST_KEEP").is_err() {
        runtime.block_on(async {
            for album in &albums {
                client.delete_album(album).await.unwrap();
            }
            if !assets.is_empty() {
                let response = reqwest::Client::new()
                    .delete(format!("{}/api/assets", url.trim_end_matches('/')))
                    .header("x-api-key", key.trim())
                    .json(&serde_json::json!({ "ids": assets, "force": true }))
                    .send()
                    .await
                    .unwrap();
                assert!(
                    response.status().is_success(),
                    "cleanup: {}",
                    response.status()
                );
            }
        });
        println!("deleted test albums {albums:?} and assets {assets:?}");
    }
    EXPORT_TARGETS.lock().unwrap().remove(&folder);
    let _ = std::fs::remove_dir_all(&folder);
    result.unwrap();
}
