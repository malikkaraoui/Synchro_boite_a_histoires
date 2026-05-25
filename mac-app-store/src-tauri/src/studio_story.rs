#![allow(dead_code)]

use serde_json::Value;
use uuid::Uuid;

const NODE_SIZE: usize = 0x2C;
const NI_HEADER_SIZE: usize = 0x200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetEntry {
    pub source_name: String,
    pub normalized_name: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StudioStory {
    pub format_version: String,
    pub pack_version: u16,
    pub title: String,
    pub description: String,
    pub factory_pack: bool,
    pub uuid: Uuid,
    pub night_mode_available: bool,
    pub compatible: bool,
    pub stage_nodes: Vec<Value>,
    pub action_nodes: Vec<Value>,
    pub ri: Vec<AssetEntry>,
    pub si: Vec<AssetEntry>,
    pub li: Vec<i32>,
}

impl StudioStory {
    pub fn from_json(story_json: &Value) -> Result<Self, String> {
        let format_version = story_json
            .get("format")
            .and_then(Value::as_str)
            .ok_or_else(|| "story.json invalide : format absent".to_string())?
            .to_string();

        let pack_version = parse_u16_field(story_json.get("version"), "version")?;
        let title = story_json
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let description = story_json
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let factory_pack = story_json
            .get("factoryPack")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let night_mode_available = story_json
            .get("nightModeAvailable")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let stage_nodes = story_json
            .get("stageNodes")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| "story.json invalide : stageNodes absent".to_string())?;
        if stage_nodes.is_empty() {
            return Err("story.json invalide : stageNodes vide".to_string());
        }

        let first_uuid = stage_nodes[0]
            .get("uuid")
            .and_then(Value::as_str)
            .ok_or_else(|| "story.json invalide : uuid absent du premier stage node".to_string())?;
        let uuid = Uuid::parse_str(first_uuid)
            .map_err(|e| format!("UUID stage node invalide : {e}"))?;

        let mut compatible = true;
        let mut ri = Vec::new();
        let mut si = Vec::new();

        for stage_node in &stage_nodes {
            if let Some(image) = stage_node.get("image").and_then(Value::as_str) {
                push_asset_if_missing(&mut ri, image);
            }
            if let Some(audio) = stage_node.get("audio").and_then(Value::as_str) {
                if !audio.to_lowercase().ends_with(".mp3") {
                    compatible = false;
                }
                push_asset_if_missing(&mut si, audio);
            }
        }

        let mut action_nodes = story_json
            .get("actionNodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut absolute_index = 0i32;
        let mut li = Vec::new();

        for action_node in &mut action_nodes {
            let Some(action_obj) = action_node.as_object_mut() else {
                return Err("story.json invalide : action node n'est pas un objet".to_string());
            };

            let options = action_obj
                .get("options")
                .and_then(Value::as_array)
                .ok_or_else(|| "story.json invalide : options absentes sur un action node".to_string())?
                .clone();

            action_obj.insert("global_index".to_string(), Value::from(absolute_index));
            absolute_index += options.len() as i32;

            for option in options {
                let option_uuid = option.as_str().ok_or_else(|| {
                    "story.json invalide : option d'action node doit être une UUID string".to_string()
                })?;
                let option_index = stage_nodes
                    .iter()
                    .position(|stage_node| {
                        stage_node
                            .get("uuid")
                            .and_then(Value::as_str)
                            .map(|uuid| uuid == option_uuid)
                            .unwrap_or(false)
                    })
                    .map(|idx| idx as i32)
                    .unwrap_or(-1);
                li.push(option_index);
            }
        }

        Ok(Self {
            format_version,
            pack_version,
            title,
            description,
            factory_pack,
            uuid,
            night_mode_available,
            compatible,
            stage_nodes,
            action_nodes,
            ri,
            si,
            li,
        })
    }

    pub fn short_uuid(&self) -> String {
        self.uuid.simple().to_string()[24..].to_uppercase()
    }

    pub fn ri_data(&self) -> Vec<u8> {
        let mut data = String::new();
        for file in &self.ri {
            data.push_str(&format!("000\\{}", file.normalized_name));
        }
        data.into_bytes()
    }

    pub fn si_data(&self) -> Vec<u8> {
        let mut data = String::new();
        for file in &self.si {
            data.push_str(&format!("000\\{}", file.normalized_name));
        }
        data.into_bytes()
    }

    pub fn li_data(&self) -> Vec<u8> {
        let mut li_buffer = Vec::new();
        for index in &self.li {
            li_buffer.extend_from_slice(&index.to_le_bytes());
        }
        while li_buffer.len() < 8 {
            li_buffer.push(0);
        }
        li_buffer
    }

    pub fn ni_data(&self) -> Result<Vec<u8>, String> {
        let mut ni_buffer = Vec::new();
        let format_num = self
            .format_version
            .strip_prefix('v')
            .ok_or_else(|| format!("Format STUdio non supporté : {}", self.format_version))?
            .parse::<u16>()
            .map_err(|e| format!("Format STUdio invalide : {e}"))?;

        ni_buffer.extend_from_slice(&format_num.to_le_bytes());
        ni_buffer.extend_from_slice(&self.pack_version.to_le_bytes());
        ni_buffer.extend_from_slice(&(NI_HEADER_SIZE as u32).to_le_bytes());
        ni_buffer.extend_from_slice(&(NODE_SIZE as u32).to_le_bytes());
        ni_buffer.extend_from_slice(&(self.stage_nodes.len() as u32).to_le_bytes());
        ni_buffer.extend_from_slice(&(self.ri.len() as u32).to_le_bytes());
        ni_buffer.extend_from_slice(&(self.si.len() as u32).to_le_bytes());
        ni_buffer.push(1);
        ni_buffer.resize(NI_HEADER_SIZE, 0);

        for stage_node in &self.stage_nodes {
            let mut current_node = Vec::new();
            let ri_index = stage_node
                .get("image")
                .and_then(Value::as_str)
                .and_then(|image| self.ri.iter().find(|entry| entry.source_name == image))
                .map(|entry| entry.index as i32)
                .unwrap_or(-1);
            let si_index = stage_node
                .get("audio")
                .and_then(Value::as_str)
                .and_then(|audio| self.si.iter().find(|entry| entry.source_name == audio))
                .map(|entry| entry.index as i32)
                .unwrap_or(-1);
            current_node.extend_from_slice(&ri_index.to_le_bytes());
            current_node.extend_from_slice(&si_index.to_le_bytes());

            append_transition_data(&mut current_node, stage_node.get("okTransition"), &self.action_nodes)?;
            append_transition_data(&mut current_node, stage_node.get("homeTransition"), &self.action_nodes)?;

            if let Some(controls) = stage_node.get("controlSettings") {
                current_node.extend_from_slice(&bool_to_u16(controls.get("wheel")).to_le_bytes());
                current_node.extend_from_slice(&bool_to_u16(controls.get("ok")).to_le_bytes());
                current_node.extend_from_slice(&bool_to_u16(controls.get("home")).to_le_bytes());
                current_node.extend_from_slice(&bool_to_u16(controls.get("pause")).to_le_bytes());
                current_node.extend_from_slice(&bool_to_u16(controls.get("autoplay")).to_le_bytes());
                current_node.extend_from_slice(&0u16.to_le_bytes());
            }

            if current_node.len() > NODE_SIZE {
                return Err("story.json invalide : node trop grand pour le format Lunii".to_string());
            }

            ni_buffer.extend_from_slice(&current_node);
            ni_buffer.extend(std::iter::repeat_n(0xAA, NODE_SIZE - current_node.len()));
        }

        Ok(ni_buffer)
    }
}

fn parse_u16_field(value: Option<&Value>, field_name: &str) -> Result<u16, String> {
    match value {
        Some(Value::String(raw)) => raw
            .parse::<u16>()
            .map_err(|e| format!("story.json invalide : {field_name} invalide ({e})")),
        Some(Value::Number(raw)) => raw
            .as_u64()
            .ok_or_else(|| format!("story.json invalide : {field_name} invalide"))
            .and_then(|v| {
                u16::try_from(v)
                    .map_err(|_| format!("story.json invalide : {field_name} hors plage"))
            }),
        Some(_) => Err(format!("story.json invalide : {field_name} invalide")),
        None => Err(format!("story.json invalide : {field_name} absent")),
    }
}

fn push_asset_if_missing(entries: &mut Vec<AssetEntry>, source_name: &str) {
    if entries.iter().any(|entry| entry.source_name == source_name) {
        return;
    }

    let normalized_name = normalize_asset_name(source_name);
    let index = entries.len();
    entries.push(AssetEntry {
        source_name: source_name.to_string(),
        normalized_name,
        index,
    });
}

fn normalize_asset_name(source_name: &str) -> String {
    let stem = source_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(source_name);
    let last_segment = stem.rsplit(['/', '\\']).next().unwrap_or(stem);
    let start = last_segment.len().saturating_sub(8);
    last_segment[start..].to_uppercase()
}

fn append_transition_data(
    buffer: &mut Vec<u8>,
    transition: Option<&Value>,
    action_nodes: &[Value],
) -> Result<(), String> {
    let Some(transition) = transition else {
        buffer.extend_from_slice(&(-1i32).to_le_bytes());
        buffer.extend_from_slice(&(-1i32).to_le_bytes());
        buffer.extend_from_slice(&(-1i32).to_le_bytes());
        return Ok(());
    };

    if transition.is_null() {
        buffer.extend_from_slice(&(-1i32).to_le_bytes());
        buffer.extend_from_slice(&(-1i32).to_le_bytes());
        buffer.extend_from_slice(&(-1i32).to_le_bytes());
        return Ok(());
    }

    let action_node_uuid = transition
        .get("actionNode")
        .and_then(Value::as_str)
        .ok_or_else(|| "story.json invalide : transition sans actionNode".to_string())?;
    let option_index = parse_i32_field(transition.get("optionIndex"), "optionIndex")?;

    let action_node = action_nodes
        .iter()
        .find(|node| node.get("id").and_then(Value::as_str) == Some(action_node_uuid))
        .ok_or_else(|| format!("story.json invalide : actionNode introuvable ({action_node_uuid})"))?;

    let global_index = parse_i32_field(action_node.get("global_index"), "global_index")?;
    let option_count = action_node
        .get("options")
        .and_then(Value::as_array)
        .map(|options| options.len() as i32)
        .ok_or_else(|| "story.json invalide : actionNode sans options".to_string())?;

    buffer.extend_from_slice(&global_index.to_le_bytes());
    buffer.extend_from_slice(&option_count.to_le_bytes());
    buffer.extend_from_slice(&option_index.to_le_bytes());
    Ok(())
}

fn parse_i32_field(value: Option<&Value>, field_name: &str) -> Result<i32, String> {
    match value {
        Some(Value::Number(raw)) => raw
            .as_i64()
            .ok_or_else(|| format!("story.json invalide : {field_name} invalide"))
            .and_then(|v| {
                i32::try_from(v)
                    .map_err(|_| format!("story.json invalide : {field_name} hors plage"))
            }),
        Some(Value::String(raw)) => raw
            .parse::<i32>()
            .map_err(|e| format!("story.json invalide : {field_name} invalide ({e})")),
        Some(_) => Err(format!("story.json invalide : {field_name} invalide")),
        None => Err(format!("story.json invalide : {field_name} absent")),
    }
}

fn bool_to_u16(value: Option<&Value>) -> u16 {
    value.and_then(Value::as_bool).unwrap_or(false) as u16
}

#[cfg(test)]
mod tests {
    use super::StudioStory;
    use serde_json::json;

    fn sample_story_json() -> serde_json::Value {
        json!({
            "format": "v1",
            "version": 1,
            "title": "Mon histoire",
            "description": "Description",
            "nightModeAvailable": true,
            "stageNodes": [
                {
                    "uuid": "11111111-1111-1111-1111-111111112222",
                    "image": "assets/cover.png",
                    "audio": "assets/intro.mp3",
                    "squareOne": true,
                    "controlSettings": {
                        "wheel": false,
                        "ok": true,
                        "home": true,
                        "pause": true,
                        "autoplay": false
                    },
                    "okTransition": {
                        "actionNode": "action-1",
                        "optionIndex": 0
                    },
                    "homeTransition": null
                },
                {
                    "uuid": "22222222-2222-2222-2222-222222223333",
                    "image": "assets/cover.png",
                    "audio": "assets/main.mp3",
                    "controlSettings": {
                        "wheel": true,
                        "ok": false,
                        "home": true,
                        "pause": true,
                        "autoplay": true
                    },
                    "okTransition": null,
                    "homeTransition": null
                }
            ],
            "actionNodes": [
                {
                    "id": "action-1",
                    "options": [
                        "22222222-2222-2222-2222-222222223333"
                    ]
                }
            ],
            "listNodes": []
        })
    }

    #[test]
    fn parses_story_json_and_collects_assets() {
        let story = StudioStory::from_json(&sample_story_json()).unwrap();

        assert_eq!(story.title, "Mon histoire");
        assert_eq!(story.short_uuid(), "11112222");
        assert!(story.compatible);
        assert_eq!(story.ri.len(), 1);
        assert_eq!(story.ri[0].normalized_name, "COVER");
        assert_eq!(story.si.len(), 2);
        assert_eq!(story.si[0].normalized_name, "INTRO");
        assert_eq!(story.si[1].normalized_name, "MAIN");
        assert_eq!(story.li, vec![1]);
    }

    #[test]
    fn marks_non_mp3_story_as_incompatible() {
        let mut story_json = sample_story_json();
        story_json["stageNodes"][0]["audio"] = json!("assets/intro.m4a");

        let story = StudioStory::from_json(&story_json).unwrap();
        assert!(!story.compatible);
    }

    #[test]
    fn generates_expected_index_buffers() {
        let story = StudioStory::from_json(&sample_story_json()).unwrap();

        assert_eq!(story.ri_data(), b"000\\COVER");
        assert_eq!(story.si_data(), b"000\\INTRO000\\MAIN");
        assert_eq!(story.li_data(), vec![1, 0, 0, 0, 0, 0, 0, 0]);

        let ni = story.ni_data().unwrap();
        assert_eq!(u16::from_le_bytes([ni[0], ni[1]]), 1);
        assert_eq!(u16::from_le_bytes([ni[2], ni[3]]), 1);
        assert_eq!(u32::from_le_bytes([ni[4], ni[5], ni[6], ni[7]]), 0x200);
        assert_eq!(u32::from_le_bytes([ni[8], ni[9], ni[10], ni[11]]), 0x2C);
        assert_eq!(u32::from_le_bytes([ni[12], ni[13], ni[14], ni[15]]), 2);
        assert_eq!(u32::from_le_bytes([ni[16], ni[17], ni[18], ni[19]]), 1);
        assert_eq!(u32::from_le_bytes([ni[20], ni[21], ni[22], ni[23]]), 2);
        assert_eq!(ni[24], 1);
        assert_eq!(ni.len(), 0x200 + 2 * 0x2C);

        let first_node = &ni[0x200..0x200 + 0x2C];
        assert_eq!(i32::from_le_bytes(first_node[0..4].try_into().unwrap()), 0);
        assert_eq!(i32::from_le_bytes(first_node[4..8].try_into().unwrap()), 0);
        assert_eq!(i32::from_le_bytes(first_node[8..12].try_into().unwrap()), 0);
        assert_eq!(i32::from_le_bytes(first_node[12..16].try_into().unwrap()), 1);
        assert_eq!(i32::from_le_bytes(first_node[16..20].try_into().unwrap()), 0);
        assert_eq!(i32::from_le_bytes(first_node[20..24].try_into().unwrap()), -1);
    }
}
