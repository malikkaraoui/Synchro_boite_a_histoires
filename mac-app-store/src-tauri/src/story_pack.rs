use png::{BitDepth, ColorType, Encoder};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[allow(dead_code)]
pub fn patch_direct_play_zip(zip_path: &Path) -> Result<bool, String> {
    let mut files = read_zip_files(zip_path)?;
    let mut story_json = read_story_json(&files)?;

    let stage_nodes = story_json
        .get("stageNodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "story.json invalide : stageNodes absent".to_string())?;

    let square_one_index = stage_nodes
        .iter()
        .position(|node| node.get("squareOne").and_then(Value::as_bool) == Some(true));
    let podcast_audio = stage_nodes
        .iter()
        .find(|node| node.get("squareOne").and_then(Value::as_bool) != Some(true))
        .and_then(|node| node.get("audio").cloned());

    let Some(square_one_index) = square_one_index else {
        return Ok(false);
    };
    let Some(podcast_audio) = podcast_audio else {
        return Ok(false);
    };

    let stage_nodes_mut = story_json
        .get_mut("stageNodes")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "story.json invalide : stageNodes absent".to_string())?;

    let mut square_one = stage_nodes_mut[square_one_index].clone();
    let square_one_obj = square_one
        .as_object_mut()
        .ok_or_else(|| "story.json invalide : squareOne n'est pas un objet".to_string())?;

    square_one_obj.insert("audio".to_string(), podcast_audio);
    square_one_obj.insert(
        "controlSettings".to_string(),
        json!({
            "autoplay": false,
            "home": true,
            "ok": false,
            "pause": true,
            "wheel": false,
        }),
    );
    square_one_obj.insert("okTransition".to_string(), Value::Null);
    square_one_obj.insert("homeTransition".to_string(), Value::Null);

    story_json["stageNodes"] = Value::Array(vec![square_one]);
    story_json["actionNodes"] = Value::Array(vec![]);
    story_json["listNodes"] = Value::Array(vec![]);

    replace_story_json(&mut files, &story_json)?;
    write_zip_files(zip_path, &files)?;
    Ok(true)
}

#[allow(dead_code)]
pub fn inject_placeholder_cover_if_missing(zip_path: &Path, title: &str) -> Result<bool, String> {
    let mut files = read_zip_files(zip_path)?;
    let mut story_json = read_story_json(&files)?;
    let stage_nodes = story_json
        .get_mut("stageNodes")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "story.json invalide : stageNodes absent".to_string())?;

    if stage_nodes.iter().any(|node| {
        node.get("image")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false)
    }) {
        return Ok(false);
    }

    let cover_bytes = create_placeholder_cover_png(title)?;
    let cover_hash = hex::encode(Sha256::digest(&cover_bytes));
    let cover_name = format!("{}.png", &cover_hash[..16]);

    for node in stage_nodes.iter_mut() {
        let Some(node_obj) = node.as_object_mut() else {
            continue;
        };
        node_obj.insert("image".to_string(), Value::String(cover_name.clone()));
    }

    files.insert(format!("assets/{cover_name}"), cover_bytes);
    replace_story_json(&mut files, &story_json)?;
    write_zip_files(zip_path, &files)?;
    Ok(true)
}

fn read_zip_files(zip_path: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let file = File::open(zip_path).map_err(|e| format!("Ouverture {:?} échouée : {e}", zip_path))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Lecture ZIP {:?} échouée : {e}", zip_path))?;

    let mut files = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| format!("Lecture entrée ZIP #{index} échouée : {e}"))?;
        if entry.is_dir() {
            continue;
        }

        let mut data = Vec::new();
        entry.read_to_end(&mut data)
            .map_err(|e| format!("Lecture contenu ZIP échouée : {e}"))?;
        files.insert(entry.name().to_string(), data);
    }

    Ok(files)
}

fn write_zip_files(zip_path: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let file = File::create(zip_path).map_err(|e| format!("Création {:?} échouée : {e}", zip_path))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for (name, data) in files {
        writer
            .start_file(name, options)
            .map_err(|e| format!("Création entrée ZIP {name} échouée : {e}"))?;
        writer
            .write_all(data)
            .map_err(|e| format!("Écriture entrée ZIP {name} échouée : {e}"))?;
    }

    writer.finish().map_err(|e| format!("Finalisation ZIP échouée : {e}"))?;
    Ok(())
}

fn read_story_json(files: &BTreeMap<String, Vec<u8>>) -> Result<Value, String> {
    let story_json = files
        .get("story.json")
        .ok_or_else(|| "story.json absent du ZIP".to_string())?;
    serde_json::from_slice(story_json).map_err(|e| format!("story.json invalide : {e}"))
}

fn replace_story_json(files: &mut BTreeMap<String, Vec<u8>>, story_json: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(story_json).map_err(|e| format!("Sérialisation story.json échouée : {e}"))?;
    files.insert("story.json".to_string(), bytes);
    Ok(())
}

fn create_placeholder_cover_png(title: &str) -> Result<Vec<u8>, String> {
    let mut png_bytes = Vec::new();
    let mut encoder = Encoder::new(&mut png_bytes, 320, 240);
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("Création entête PNG échouée : {e}"))?;

    let digest = Sha256::digest(title.as_bytes());
    let color = [20 ^ digest[0], 50 ^ digest[1], 120 ^ digest[2]];
    let mut row = vec![0u8; 320 * 3];
    for pixel in row.chunks_exact_mut(3) {
        pixel.copy_from_slice(&color);
    }

    let mut image = Vec::with_capacity(row.len() * 240);
    for _ in 0..240 {
        image.extend_from_slice(&row);
    }

    writer
        .write_image_data(&image)
        .map_err(|e| format!("Écriture PNG échouée : {e}"))?;
    drop(writer);
    Ok(png_bytes)
}

#[cfg(test)]
mod tests {
    use super::{inject_placeholder_cover_if_missing, patch_direct_play_zip, read_zip_files};
    use serde_json::{json, Value};
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock drift")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn create_test_zip(path: &Path, story_json: Value) {
        let file = File::create(path).expect("create zip");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        writer.start_file("story.json", options).unwrap();
        writer.write_all(serde_json::to_string(&story_json).unwrap().as_bytes()).unwrap();
        writer.start_file("assets/existing.txt", options).unwrap();
        writer.write_all(b"keep me").unwrap();
        writer.finish().unwrap();
    }

    fn read_story_json(path: &Path) -> Value {
        let files = read_zip_files(path).unwrap();
        serde_json::from_slice(files.get("story.json").unwrap()).unwrap()
    }

    #[test]
    fn patch_direct_play_rewrites_story_json() {
        let tmp = TempDir::new("luniisync-story-pack-patch");
        let zip_path = tmp.path().join("story.zip");
        create_test_zip(
            &zip_path,
            json!({
                "stageNodes": [
                    {"squareOne": true, "audio": "intro.mp3"},
                    {"squareOne": false, "audio": "podcast.mp3"}
                ],
                "actionNodes": [{"id": 1}],
                "listNodes": [{"id": 2}]
            }),
        );

        let changed = patch_direct_play_zip(&zip_path).unwrap();
        assert!(changed);

        let story_json = read_story_json(&zip_path);
        let stage_nodes = story_json["stageNodes"].as_array().unwrap();
        assert_eq!(stage_nodes.len(), 1);
        assert_eq!(stage_nodes[0]["audio"], "podcast.mp3");
        assert_eq!(story_json["actionNodes"], json!([]));
        assert_eq!(story_json["listNodes"], json!([]));
        assert_eq!(stage_nodes[0]["controlSettings"]["home"], true);
        assert!(stage_nodes[0]["okTransition"].is_null());
        assert!(stage_nodes[0]["homeTransition"].is_null());
    }

    #[test]
    fn inject_placeholder_cover_adds_png_and_updates_nodes() {
        let tmp = TempDir::new("luniisync-story-pack-cover");
        let zip_path = tmp.path().join("story.zip");
        create_test_zip(
            &zip_path,
            json!({
                "stageNodes": [
                    {"squareOne": true, "audio": "intro.mp3"},
                    {"squareOne": false, "audio": "podcast.mp3"}
                ],
                "actionNodes": [],
                "listNodes": []
            }),
        );

        let changed = inject_placeholder_cover_if_missing(&zip_path, "Mon histoire").unwrap();
        assert!(changed);

        let files = read_zip_files(&zip_path).unwrap();
        let story_json: Value = serde_json::from_slice(files.get("story.json").unwrap()).unwrap();
        let stage_nodes = story_json["stageNodes"].as_array().unwrap();
        let image_name = stage_nodes[0]["image"].as_str().unwrap().to_string();
        assert!(image_name.ends_with(".png"));
        assert_eq!(stage_nodes[1]["image"].as_str(), Some(image_name.as_str()));
        assert!(files.contains_key(&format!("assets/{image_name}")));
    }

    #[test]
    fn inject_placeholder_cover_does_nothing_when_image_exists() {
        let tmp = TempDir::new("luniisync-story-pack-existing-cover");
        let zip_path = tmp.path().join("story.zip");
        create_test_zip(
            &zip_path,
            json!({
                "stageNodes": [
                    {"squareOne": true, "audio": "intro.mp3", "image": "already.png"},
                    {"squareOne": false, "audio": "podcast.mp3"}
                ],
                "actionNodes": [],
                "listNodes": []
            }),
        );

        let changed = inject_placeholder_cover_if_missing(&zip_path, "Mon histoire").unwrap();
        assert!(!changed);

        let files = read_zip_files(&zip_path).unwrap();
        assert!(!files.keys().any(|name| name.starts_with("assets/") && name.ends_with(".png") && name != "assets/existing.txt"));
    }
}
