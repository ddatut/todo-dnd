use crate::dm::{self, TaleRequest};
use crate::state::{self, Character, Companion, Task};
use rand::Rng;

pub struct ModuleIntro {
    pub name: String,
    pub level: String,
    pub summary: String,
    pub first_beat: String,
}

pub struct PendingTale {
    pub task_desc: String,
    pub roll: u32,
    pub sides: u32,
    pub check_name: String,
    pub modifier: i8,
    pub total: i32,
    pub encounter: Option<String>,
    pub hp_note: Option<String>,
    pub progress: Option<String>,
    pub loyalty_note: Option<String>,
    pub quest_complete: Option<String>,
    ctx: TaleRequest,
}

pub struct TaleResult {
    pub narrative: String,
    pub image_path: Option<String>,
    pub image_missing: bool,
}

pub fn add_task(description: String, category: Option<String>) {
    state::with_save_lock(|| {
        let mut tasks = state::load_tasks();
        tasks.push(Task {
            description,
            completed: false,
            category,
            quest_id: None,
            quest_step: None,
        });
        state::save_tasks(&tasks);
    });
}

pub fn add_quest(name: &str, steps: &str) -> Result<usize, String> {
    let step_list: Vec<&str> = steps
        .split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if step_list.is_empty() {
        return Err("Provide steps separated by |".to_string());
    }
    let count = step_list.len();
    state::with_save_lock(|| {
        let mut tasks = state::load_tasks();
        for (i, step) in step_list.iter().enumerate() {
            tasks.push(Task {
                description: (*step).to_string(),
                completed: false,
                category: None,
                quest_id: Some(name.to_string()),
                quest_step: Some(i + 1),
            });
        }
        state::save_tasks(&tasks);
    });
    Ok(count)
}

pub fn mark_complete(index: usize) -> Result<Task, String> {
    state::with_save_lock(|| {
        let mut tasks = state::load_tasks();
        let task = tasks
            .get_mut(index)
            .ok_or_else(|| "Invalid task index. Use `list` to see them.".to_string())?;
        if task.completed {
            return Err("That task is already completed.".to_string());
        }
        task.completed = true;
        let snapshot = task.clone();
        state::save_tasks(&tasks);
        Ok(snapshot)
    })
}

pub fn reopen_task(index: usize) -> Result<(), String> {
    state::with_save_lock(|| {
        let mut tasks = state::load_tasks();
        let task = tasks
            .get_mut(index)
            .ok_or_else(|| "Invalid task index. Use `list` to see them.".to_string())?;
        task.completed = false;
        state::save_tasks(&tasks);
        Ok(())
    })
}

pub fn clear_tasks(all: bool) {
    state::with_save_lock(|| {
        let mut tasks = state::load_tasks();
        if all {
            tasks.clear();
        } else {
            tasks.retain(|t| !t.completed);
        }
        state::save_tasks(&tasks);
    });
}

/// Roll, apply hit points, beat progress, and encounters, then return the snapshot the DM should see.
pub fn begin_tale(task: Task, sides: u32) -> PendingTale {
    let sides = sides.max(2);
    state::with_save_lock(|| {
        let mut campaign = state::load_campaign();
        let mut character = state::load_character();
        let tasks = state::load_tasks();

        let mut rng = rand::thread_rng();
        let roll = rng.gen_range(1..=sides);
        let (check_name, modifier) = state::category_to_check(&task.category, &character);
        let total = roll as i32 + i32::from(modifier);

        let ctx = TaleRequest {
            task: task.description.clone(),
            roll,
            sides,
            modifier,
            check_name: check_name.to_string(),
            scene: campaign.current_scene.clone(),
            active_module: campaign.active_module.clone(),
            character: character.clone(),
            progress_note: campaign.progress_note.clone(),
            current_beat: campaign.current_beat,
            recent: campaign.recent_summaries.clone(),
            companion: campaign.companion.clone(),
        };

        let encounter = (roll <= 8).then(|| {
            let enc = state::random_encounter(campaign.active_module.as_deref());
            campaign
                .events
                .push(format!("Random encounter: {enc}"));
            enc.to_string()
        });

        let history_entry = if check_name != "general" {
            format!(
                "{}: d{sides} = {roll} {modifier:+} = {total} ({check_name})",
                task.description
            )
        } else {
            format!("{}: d{sides} = {roll} (general)", task.description)
        };
        campaign.dice_history.push(history_entry);
        if campaign.dice_history.len() > 20 {
            campaign.dice_history.remove(0);
        }

        let mut progress = None;
        if let Some(id) = campaign.active_module.clone() {
            if let Some(module) = state::get_module(&id) {
                if total >= sides as i32 / 2 && campaign.current_beat + 1 < module.beats.len() {
                    campaign.current_beat += 1;
                    let new_beat = module.beats[campaign.current_beat];
                    campaign.progress_note =
                        format!("Beat {}: {new_beat}", campaign.current_beat + 1);
                    campaign.current_scene = new_beat.to_string();
                    if !campaign.explored_areas.iter().any(|area| area == new_beat) {
                        campaign.explored_areas.push(new_beat.to_string());
                    }
                    progress = Some(new_beat.to_string());
                }
            }
        }

        if !campaign.current_scene.is_empty()
            && !campaign
                .explored_areas
                .iter()
                .any(|area| area == &campaign.current_scene)
        {
            campaign
                .explored_areas
                .push(campaign.current_scene.clone());
        }

        let hp_note = if roll <= 5 {
            character.hp = (character.hp - 3).max(0);
            Some(format!(
                "The struggle costs you 3 HP (now {}/{}).",
                character.hp, character.max_hp
            ))
        } else if roll >= sides.saturating_sub(1) && character.hp < character.max_hp {
            character.hp = (character.hp + 2).min(character.max_hp);
            Some(format!(
                "Success invigorates you (+2 HP, now {}/{}).",
                character.hp, character.max_hp
            ))
        } else {
            None
        };

        let mut loyalty_note = None;
        if let Some(companion) = campaign.companion.as_mut() {
            let before = companion.loyalty;
            if roll <= 5 {
                companion.loyalty = (companion.loyalty - 1).max(0);
            } else if total >= sides as i32 / 2 {
                companion.loyalty = (companion.loyalty + 1).min(5);
            }
            if companion.loyalty != before {
                loyalty_note = Some(format!(
                    "{}'s loyalty is now {}.",
                    companion.name, companion.loyalty
                ));
            }
        }

        if campaign.current_scene.is_empty() {
            campaign.current_scene = "deep within the dungeon".to_string();
        }
        if campaign.progress_note.is_empty() {
            campaign.progress_note = "Adventure underway".to_string();
        }

        let quest_complete = task.quest_id.as_ref().and_then(|qid| {
            let remaining = tasks
                .iter()
                .filter(|t| t.quest_id.as_deref() == Some(qid.as_str()) && !t.completed)
                .count();
            if remaining == 0 {
                campaign.events.push(format!("Quest completed: {qid}"));
                Some(qid.clone())
            } else {
                None
            }
        });

        state::save_campaign(&campaign);
        state::save_character(&character);

        PendingTale {
            task_desc: task.description,
            roll,
            sides,
            check_name: check_name.to_string(),
            modifier,
            total,
            encounter,
            hp_note,
            progress,
            loyalty_note,
            quest_complete,
            ctx,
        }
    })
}

pub async fn finish_tale(pending: PendingTale, no_image: bool) -> TaleResult {
    let narrative = dm::ai_dungeon_master(&pending.ctx).await;
    let has_key = std::env::var("OPENAI_API_KEY")
        .ok()
        .is_some_and(|key| !key.is_empty());
    let image_path = if !no_image && has_key {
        dm::generate_illustration(&narrative, &pending.task_desc).await
    } else {
        None
    };

    state::with_save_lock(|| {
        let mut campaign = state::load_campaign();
        if let Some(path) = &image_path {
            campaign.images.push(path.clone());
            state::unlock_achievement(&mut campaign, "First illustration");
        }
        let event = format!(
            "Completed \"{}\" (roll {}/{}): {}{}",
            pending.task_desc,
            pending.roll,
            pending.sides,
            narrative.lines().next().unwrap_or("Adventure continued."),
            image_path
                .as_ref()
                .map(|path| format!(" [image: {path}]"))
                .unwrap_or_default()
        );
        campaign.events.push(event);
        let summary = narrative
            .lines()
            .next()
            .unwrap_or("Adventure continued.")
            .to_string();
        campaign.recent_summaries.push(summary);
        if campaign.recent_summaries.len() > 3 {
            campaign.recent_summaries.remove(0);
        }
        campaign.last_narrative = Some(narrative.clone());
        state::unlock_achievement(&mut campaign, "First task completed");
        if campaign.explored_areas.len() >= 3 {
            state::unlock_achievement(&mut campaign, "Explorer");
        }
        if campaign.dice_history.len() >= 5 {
            state::unlock_achievement(&mut campaign, "Seasoned roller");
        }
        if let Some(qid) = &pending.quest_complete {
            state::unlock_achievement(&mut campaign, &format!("Completed quest: {qid}"));
        }
        state::save_campaign(&campaign);
    });

    TaleResult {
        narrative,
        image_missing: !no_image && has_key && image_path.is_none(),
        image_path,
    }
}

pub fn use_item(query: &str) -> Result<String, String> {
    state::with_save_lock(|| {
        let mut character = state::load_character();
        let pos = character
            .inventory
            .iter()
            .position(|item| item.to_lowercase().contains(&query.to_lowercase()));
        let Some(pos) = pos else {
            return Err(format!(
                "No matching item in inventory. Current items: {}",
                character.inventory.join(", ")
            ));
        };
        let used = character.inventory.remove(pos);
        let effect = state::item_effect(&used);
        let message = match effect {
            "healing" => {
                character.hp = (character.hp + 5).min(character.max_hp);
                format!(
                    "Used {used}. Restored 5 HP (now {}/{}).",
                    character.hp, character.max_hp
                )
            }
            "light" => format!("Used {used}. The area is better illuminated."),
            "tool" => format!("Used {used}. It gives you an edge on the next challenge."),
            "weapon" => format!("Used {used}. You feel better prepared for combat."),
            _ => format!("Used {used}."),
        };
        state::save_character(&character);
        let mut campaign = state::load_campaign();
        campaign
            .events
            .push(format!("Used item: {used} (effect: {effect})"));
        state::save_campaign(&campaign);
        Ok(message)
    })
}

pub fn add_item(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Item name is empty.".to_string());
    }
    let name = name.to_string();
    state::modify_character(|character| {
        character.inventory.push(name);
    });
    Ok(())
}

pub fn adjust_hp(amount: i32) -> String {
    state::modify_character(|character| {
        character.hp = (character.hp + amount).clamp(0, character.max_hp);
        format!(
            "HP adjusted by {amount}. Now {}/{}.",
            character.hp, character.max_hp
        )
    })
}

pub fn rest() -> (i32, i32, i32) {
    state::with_save_lock(|| {
        let mut character = state::load_character();
        character.hp = character.hp.clamp(0, character.max_hp);
        let restored = (character.max_hp - character.hp).clamp(0, 5);
        character.hp += restored;
        let hp = character.hp;
        let max_hp = character.max_hp;
        state::save_character(&character);
        let mut campaign = state::load_campaign();
        campaign
            .events
            .push(format!("Rested and recovered {restored} HP."));
        campaign.current_scene = "A quiet place to rest".to_string();
        state::save_campaign(&campaign);
        (restored, hp, max_hp)
    })
}

pub fn set_companion(name: String, personality: String) {
    state::with_save_lock(|| {
        let mut campaign = state::load_campaign();
        campaign.companion = Some(Companion {
            name,
            personality,
            loyalty: 1,
        });
        state::save_campaign(&campaign);
    });
}

pub fn add_note(text: String) {
    state::with_save_lock(|| {
        let mut campaign = state::load_campaign();
        campaign.notes.push(text.clone());
        campaign.events.push(format!("Player note: {text}"));
        state::save_campaign(&campaign);
    });
}

pub fn start_module(id: &str) -> Result<ModuleIntro, String> {
    let module = state::get_module(id).ok_or_else(|| {
        format!(
            "Unknown module '{id}'. Available:\n{}",
            state::module_list_text()
        )
    })?;
    let first = module.beats.first().copied().unwrap_or("start");
    state::with_save_lock(|| {
        let mut campaign = state::load_campaign();
        campaign.active_module = Some(id.to_string());
        campaign.current_scene = format!("Beginning of {}", module.name);
        campaign.current_beat = 0;
        campaign.progress_note = format!("Beat 1: {first}");
        campaign.explored_areas.push(first.to_string());
        campaign
            .events
            .push(format!("Started campaign module: {}", module.name));
        state::save_campaign(&campaign);
    });
    Ok(ModuleIntro {
        name: module.name.to_string(),
        level: module.level.to_string(),
        summary: module.summary.to_string(),
        first_beat: first.to_string(),
    })
}

pub fn save_sheet(sheet: &Character, companion_name: &str, companion_personality: &str) {
    state::with_save_lock(|| {
        let mut character = state::load_character();
        character.name = sheet.name.clone();
        character.class = sheet.class.clone();
        character.level = sheet.level;
        character.background = sheet.background.clone();
        character.feature = sheet.feature.clone();
        character.goal = sheet.goal.clone();
        character.str_mod = sheet.str_mod;
        character.dex_mod = sheet.dex_mod;
        character.con_mod = sheet.con_mod;
        character.int_mod = sheet.int_mod;
        character.wis_mod = sheet.wis_mod;
        character.cha_mod = sheet.cha_mod;
        state::save_character(&character);

        let mut campaign = state::load_campaign();
        let name = companion_name.trim();
        if name.is_empty() {
            campaign.companion = None;
        } else {
            let loyalty = campaign
                .companion
                .as_ref()
                .filter(|c| c.name == name)
                .map(|c| c.loyalty)
                .unwrap_or(1);
            campaign.companion = Some(Companion {
                name: name.to_string(),
                personality: companion_personality.trim().to_string(),
                loyalty,
            });
        }
        state::save_campaign(&campaign);
    });
}

pub async fn draw_map() -> Result<String, String> {
    let campaign = state::with_save_lock(state::load_campaign);
    if campaign.explored_areas.is_empty() && campaign.current_scene.is_empty() {
        return Err(
            "No areas explored yet. Complete a task or start a module first.".to_string(),
        );
    }
    let path = dm::generate_map(&campaign)
        .await
        .ok_or_else(|| "Could not generate map — check API key / IMAGE_MODEL".to_string())?;
    state::with_save_lock(|| {
        let mut campaign = state::load_campaign();
        campaign.map_path = Some(path.clone());
        campaign.images.push(path.clone());
        state::unlock_achievement(&mut campaign, "First map drawn");
        state::save_campaign(&campaign);
    });
    Ok(path)
}

pub fn export_log(path: &str) -> Result<(), String> {
    let campaign = state::load_campaign();
    let character = state::load_character();
    let mut content = format!(
        "# Session Log\n\n**Character:** {} the level {} {}\n\n",
        character.name, character.level, character.class
    );
    if let Some(id) = &campaign.active_module {
        if let Some(module) = state::get_module(id) {
            content.push_str(&format!("**Module:** {} ({})\n\n", module.name, module.level));
        }
    }
    content.push_str(&format!("**Current scene:** {}\n\n", campaign.current_scene));
    if !campaign.progress_note.is_empty() {
        content.push_str(&format!("**Progress:** {}\n\n", campaign.progress_note));
    }
    if let Some(companion) = &campaign.companion {
        content.push_str(&format!(
            "**Companion:** {} — {} (loyalty {})\n\n",
            companion.name, companion.personality, companion.loyalty
        ));
    }
    content.push_str("## Events\n\n");
    for event in &campaign.events {
        content.push_str(&format!("- {event}\n"));
    }
    if !campaign.images.is_empty() {
        content.push_str("\n## Illustrations\n\n");
        for img in &campaign.images {
            content.push_str(&format!("- {img}\n"));
        }
    }
    std::fs::write(path, content).map_err(|_| format!("Failed to write {path}"))
}

pub async fn tell(task: Task, sides: u32, no_image: bool) -> TaleResult {
    let pending = begin_tale(task, sides);
    finish_tale(pending, no_image).await
}
