use crate::state::{get_module, Character, Companion};
use std::fs;
use std::path::PathBuf;

pub const IMAGES_DIR: &str = "images";

#[derive(Clone)]
pub struct TaleRequest {
    pub task: String,
    pub roll: u32,
    pub sides: u32,
    pub modifier: i8,
    pub check_name: String,
    pub scene: String,
    pub active_module: Option<String>,
    pub character: Character,
    pub progress_note: String,
    pub current_beat: usize,
    pub recent: Vec<String>,
    pub companion: Option<Companion>,
}

pub async fn ai_dungeon_master(request: &TaleRequest) -> String {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        eprintln!("Warning: OPENAI_API_KEY not set. Using placeholder narrative.");
        String::new()
    });

    let total = request.roll as i32 + request.modifier as i32;

    if api_key.is_empty() {
        return offline_narrative(request, total);
    }

    let base_url =
        std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.x.ai/v1".to_string());
    let model = std::env::var("MODEL").unwrap_or_else(|_| "grok-2".to_string());

    let client = reqwest::Client::new();
    let module_context = if let Some(id) = &request.active_module {
        if let Some(m) = get_module(id) {
            let beat = m
                .beats
                .get(request.current_beat)
                .unwrap_or(&"the adventure");
            format!(
                "\n\nACTIVE CAMPAIGN MODULE: {}\nSummary: {}\nGuidance: {}\nCurrent beat ({}): {}.\nStay consistent with this module's tone, key NPCs, and major beats. Advance the story naturally based on the player's actions and roll.",
                m.name,
                m.summary,
                m.dm_guidance,
                request.current_beat + 1,
                beat
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let character = &request.character;
    let character_context = format!(
        "\nCharacter: {} the level {} {} (HP {}/{}). Background: {}. Feature: {}. Goal: {}. Inventory: {}. Relevant check: {} (modifier {:+}).",
        character.name,
        character.level,
        character.class,
        character.hp,
        character.max_hp,
        if character.background.is_empty() { "unknown" } else { &character.background },
        if character.feature.is_empty() { "none" } else { &character.feature },
        if character.goal.is_empty() { "survive and thrive" } else { &character.goal },
        if character.inventory.is_empty() { "none".to_string() } else { character.inventory.join(", ") },
        request.check_name,
        request.modifier
    );
    let progress_context = if request.progress_note.is_empty() {
        String::new()
    } else {
        format!("\nProgress note: {}", request.progress_note)
    };
    let memory = if request.recent.is_empty() {
        String::new()
    } else {
        let lines = request
            .recent
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\nRecent scene memory:\n{lines}")
    };
    let companion_context = request
        .companion
        .as_ref()
        .map(|c| {
            format!(
                "\nCompanion: {} — {}. Loyalty {}.",
                c.name, c.personality, c.loyalty
            )
        })
        .unwrap_or_default();

    let scene = if request.scene.is_empty() {
        "starting a new adventure in a mysterious dungeon"
    } else {
        request.scene.as_str()
    };
    let prompt = format!(
        "You are a witty, engaging Dungeon Master for a solo D&D campaign. \
         The player's current scene is: {scene}.{character_context}{companion_context}{progress_context}{memory}\n\
         They just completed a real-world task: \"{}\". \
         Natural roll: {} on a d{}. Modifier: {:+}. Total: {total}. Treat this as a {} check. \
         Higher is better.\n\
         Narrate 2-3 short paragraphs tying the completed task to an in-game action or discovery. \
         Describe the outcome based on the total (a strong success is near the top of the die, a failure is under half the die), \
         advance the story slightly, and end with a hook for the next action. \
         If a companion is present, let them react in character. Keep it fun and immersive. No meta commentary.{module_context}",
        request.task, request.roll, request.sides, request.modifier, request.check_name
    );

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are an expert Dungeon Master running a solo D&D game. Be creative, concise, and fun."},
            {"role": "user", "content": prompt}
        ],
        "max_tokens": 400,
        "temperature": 0.8
    });

    match client
        .post(format!("{}/chat/completions", base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                        return content.trim().to_string();
                    }
                }
            }
            format!(
                "The DM's magic fizzled (API error). Fallback: Completing \"{}\" with a roll of {} lets you push forward through the shadows.",
                request.task, request.roll
            )
        }
        Err(e) => format!(
            "Connection to the DM failed: {e}. Completing \"{}\" still advances you somehow.",
            request.task
        ),
    }
}

fn offline_narrative(request: &TaleRequest, total: i32) -> String {
    let success = if total >= request.sides as i32 / 2 {
        "succeed"
    } else {
        "struggle"
    };
    let beat_info = request
        .active_module
        .as_deref()
        .and_then(get_module)
        .and_then(|m| m.beats.get(request.current_beat).copied())
        .map(|beat| format!(" Current beat: {beat}."))
        .unwrap_or_default();
    let companion_bit = request
        .companion
        .as_ref()
        .map(|c| format!(" {} stays close.", c.name))
        .unwrap_or_default();
    let check = if request.check_name == "general" {
        format!("a roll of {}", request.roll)
    } else {
        format!(
            "{} ({} {:+}, total {total})",
            request.roll, request.check_name, request.modifier
        )
    };
    let scene = if request.scene.is_empty() {
        "shadowy dungeon"
    } else {
        request.scene.as_str()
    };
    let memory = request
        .recent
        .last()
        .map(|s| format!(" You still remember: {s}"))
        .unwrap_or_default();
    format!(
        "{} the level {} {} completes \"{}\" and gets {check} on a d{}. You {success}!{beat_info}{companion_bit}{memory}\n\
         The Dungeon Master narrates: In the {scene}, your actions create an opening. \
         The shadows shift and a new path (or danger) reveals itself. \
         What do you do next? (Complete more tasks to continue the adventure!)",
        request.character.name,
        request.character.level,
        request.character.class,
        request.task,
        request.sides
    )
}

/// Generate a handmade pen-and-ink illustration of a key moment from the narrative.
/// Saves the image locally and returns the relative path on success.
pub async fn generate_illustration(narrative: &str, task: &str) -> Option<String> {
    let api_key = std::env::var("OPENAI_API_KEY").ok()?;
    if api_key.is_empty() {
        return None;
    }

    let base_url =
        std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.x.ai/v1".to_string());
    let image_model =
        std::env::var("IMAGE_MODEL").unwrap_or_else(|_| "grok-imagine-image-2.0".to_string());

    let scene_hint = narrative
        .lines()
        .find(|l| l.trim().len() > 40)
        .unwrap_or(narrative)
        .chars()
        .take(280)
        .collect::<String>();

    let image_prompt = format!(
        "Handmade pen and ink drawing, black ink on textured cream sketchbook paper, \
         visible paper grain, imperfect hand-drawn lines, cross-hatching for shadows, \
         classic fantasy illustration style, monochrome, no color, no digital polish. \
         Illustrate this key moment from a D&D adventure: {}. \
         The scene is inspired by the real-world action of \"{task}\". \
         Focus on one strong, clear composition with generous white space.",
        scene_hint.trim()
    );

    download_image(&api_key, &base_url, &image_model, &image_prompt, "scene").await
}

/// Generate a pen-and-ink campaign map of explored areas (with hints of the unknown).
pub async fn generate_map(campaign: &crate::state::Campaign) -> Option<String> {
    let api_key = std::env::var("OPENAI_API_KEY").ok()?;
    if api_key.is_empty() {
        return None;
    }

    let base_url =
        std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.x.ai/v1".to_string());
    let image_model =
        std::env::var("IMAGE_MODEL").unwrap_or_else(|_| "grok-imagine-image-2.0".to_string());

    let explored = if campaign.explored_areas.is_empty() {
        "the starting area".to_string()
    } else {
        campaign.explored_areas.join(", ")
    };

    let mut next_hints = String::new();
    if let Some(id) = &campaign.active_module {
        if let Some(m) = get_module(id) {
            let future: Vec<_> = m
                .beats
                .iter()
                .skip(campaign.current_beat + 1)
                .take(2)
                .cloned()
                .collect();
            if !future.is_empty() {
                next_hints = format!(
                    " Unexplored regions hinted at the edges: {}.",
                    future.join(" and ")
                );
            }
        }
    }

    let map_prompt = format!(
        "Handmade pen and ink fantasy campaign map, black ink on textured cream parchment paper, \
         visible paper grain, imperfect hand-drawn cartography, cross-hatching for terrain, \
         classic D&D adventure map style, monochrome, no color, no digital polish. \
         Top-down regional or dungeon map showing the areas already explored: {explored}. \
         Include a simple compass rose, paths connecting known locations, and small symbolic icons for key places. \
         Leave generous blank space and faint unfinished lines at the edges to suggest unexplored territory.{next_hints} \
         Focus on clear, readable composition with lots of white space."
    );

    download_image(&api_key, &base_url, &image_model, &map_prompt, "map").await
}

async fn download_image(
    api_key: &str,
    base_url: &str,
    image_model: &str,
    prompt: &str,
    prefix: &str,
) -> Option<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": image_model,
        "prompt": prompt,
        "n": 1,
        "response_format": "url",
        "aspect_ratio": "4:3"
    });

    let resp = match client
        .post(format!(
            "{}/images/generations",
            base_url.trim_end_matches('/')
        ))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Image generation request failed: {e}");
            return None;
        }
    };

    if !resp.status().is_success() {
        eprintln!("Image API returned error status: {}", resp.status());
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let image_url = json["data"][0]["url"].as_str()?;
    let img_bytes = match client.get(image_url).send().await {
        Ok(r) if r.status().is_success() => r.bytes().await.ok()?,
        _ => return None,
    };

    let _ = fs::create_dir_all(IMAGES_DIR);
    let filename = format!("{prefix}_{}.png", timestamp());
    let path = PathBuf::from(IMAGES_DIR).join(&filename);
    fs::write(&path, &img_bytes).ok()?;
    Some(format!("{IMAGES_DIR}/{filename}"))
}

fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .to_string()
}
