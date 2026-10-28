use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

const TODO_FILE: &str = "todos.json";
const CAMPAIGN_FILE: &str = "campaign.json";
const CHARACTER_FILE: &str = "character.json";

static FILE_LOCK: Mutex<()> = Mutex::new(());

pub fn with_save_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = lock();
    f()
}

fn lock() -> MutexGuard<'static, ()> {
    FILE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub description: String,
    pub completed: bool,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub quest_id: Option<String>,
    #[serde(default)]
    pub quest_step: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Character {
    pub name: String,
    pub class: String,
    pub level: u8,
    pub str_mod: i8,
    pub dex_mod: i8,
    pub con_mod: i8,
    pub int_mod: i8,
    pub wis_mod: i8,
    pub cha_mod: i8,
    pub hp: i32,
    pub max_hp: i32,
    pub inventory: Vec<String>,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub feature: String,
    #[serde(default)]
    pub goal: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Companion {
    pub name: String,
    pub personality: String,
    pub loyalty: i32,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Campaign {
    pub events: Vec<String>,
    pub current_scene: String,
    pub images: Vec<String>,
    pub active_module: Option<String>,
    #[serde(default)]
    pub progress_note: String,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub dice_history: Vec<String>,
    #[serde(default)]
    pub current_beat: usize,
    #[serde(default)]
    pub explored_areas: Vec<String>,
    #[serde(default)]
    pub map_path: Option<String>,
    #[serde(default)]
    pub recent_summaries: Vec<String>,
    #[serde(default)]
    pub achievements: Vec<String>,
    #[serde(default)]
    pub companion: Option<Companion>,
    #[serde(default)]
    pub last_narrative: Option<String>,
}

pub struct CampaignModule {
    pub id: &'static str,
    pub name: &'static str,
    pub level: &'static str,
    pub summary: &'static str,
    pub dm_guidance: &'static str,
    pub beats: &'static [&'static str],
}

const MODULES: &[CampaignModule] = &[
    CampaignModule {
        id: "wild-sheep",
        name: "The Wild Sheep Chase",
        level: "4-5 (or adaptable)",
        summary: "A frantic sheep bursts into the tavern carrying a Scroll of Speak with Animals. It is actually the wizard Finethir Shinebright, polymorphed by his jealous apprentice Ahmed Noke, who has seized the Wand of True Polymorph.",
        dm_guidance: "Run this as a fun, pun-filled one-shot. Key beats: tavern interruption by the sheep, learning the story, traveling to the tower, dealing with transmuted assassins/creatures, confronting Noke, recovering the wand, and restoring Finethir. Tie real-world completed tasks to clever solutions, skill checks, or combat moments. Keep the tone light and absurd.",
        beats: &[
            "Tavern interruption by the sheep",
            "Learning Finethir's story",
            "Travel to the tower",
            "Dealing with transmuted creatures",
            "Confronting Noke",
            "Recovering the wand and restoring Finethir",
        ],
    },
    CampaignModule {
        id: "delian",
        name: "The Delian Tomb",
        level: "1-3",
        summary: "Goblins have raided a nearby village and taken a prisoner into an old tomb of a forgotten knightly order. The party must clear the goblins, solve a riddle, avoid a trap, find a secret door, and rescue the captive (with optional undead knights if they disturb the sarcophagi).",
        dm_guidance: "Classic introductory dungeon crawl. Structure: approach the tomb, goblin lookouts/patrol, main goblin room, boss, riddle on a statue that opens a secret door (Delian Oath), optional skeletons if they loot the wrong way, and a magic sword reward. Use completed tasks as the actions that let the player advance rooms, make skill checks, or win fights. Keep it straightforward and heroic.",
        beats: &[
            "Approach the tomb",
            "Goblin patrol / lookouts",
            "Main goblin chamber",
            "Goblin boss fight",
            "Statue riddle and secret door",
            "Secret chamber and rescue / treasure",
        ],
    },
    CampaignModule {
        id: "death-house",
        name: "Death House",
        level: "1-3",
        summary: "Two pale children (Rose and Thorn) stand outside a fog-bound manor and beg the party to enter and deal with a monster in the basement. The house is a gothic horror trap full of undead, secrets of the Durst family cult, and a living house that does not want its guests to leave.",
        dm_guidance: "Gothic horror one-shot / intro. Key elements: the illusory children, exploring the haunted house room by room, uncovering the Durst family's dark history, discovering the true nature of Rose and Thorn, descending to the dungeon level, and facing the house's final demand for a sacrifice. Atmosphere is tense, creepy, and tragic. Tie completed tasks to exploring rooms, succeeding at checks, or surviving encounters. End scenes with a lingering sense of dread.",
        beats: &[
            "Meeting Rose and Thorn outside",
            "Entering the manor",
            "Exploring the upper floors",
            "Discovering the family's dark history",
            "Descending to the dungeon",
            "Final confrontation and escape",
        ],
    },
];

pub fn modules() -> &'static [CampaignModule] {
    MODULES
}

pub fn get_module(id: &str) -> Option<&'static CampaignModule> {
    MODULES.iter().find(|m| m.id == id)
}

pub fn module_list_text() -> String {
    MODULES
        .iter()
        .map(|m| format!("{} ({}) — {}", m.id, m.level, m.name))
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_file(path: &str, data: &str) {
    fs::write(path, data).unwrap();
}

pub fn load_tasks() -> Vec<Task> {
    if Path::new(TODO_FILE).exists() {
        let data = fs::read_to_string(TODO_FILE).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn save_tasks(tasks: &[Task]) {
    write_file(TODO_FILE, &serde_json::to_string_pretty(tasks).unwrap());
}

pub fn load_campaign() -> Campaign {
    if Path::new(CAMPAIGN_FILE).exists() {
        let data = fs::read_to_string(CAMPAIGN_FILE).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Campaign::default()
    }
}

pub fn save_campaign(campaign: &Campaign) {
    write_file(CAMPAIGN_FILE, &serde_json::to_string_pretty(campaign).unwrap());
}

pub fn load_character() -> Character {
    if Path::new(CHARACTER_FILE).exists() {
        let data = fs::read_to_string(CHARACTER_FILE).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_else(|_| fresh_character())
    } else {
        fresh_character()
    }
}

pub fn fresh_character() -> Character {
    Character {
        name: "Adventurer".to_string(),
        class: "Fighter".to_string(),
        level: 1,
        str_mod: 2,
        dex_mod: 1,
        con_mod: 1,
        int_mod: 0,
        wis_mod: 0,
        cha_mod: 0,
        hp: 12,
        max_hp: 12,
        inventory: vec!["Torch".to_string(), "Rope (50 ft)".to_string()],
        ..Character::default()
    }
}

pub fn save_character(character: &Character) {
    write_file(
        CHARACTER_FILE,
        &serde_json::to_string_pretty(character).unwrap(),
    );
}

pub fn modify_character<T>(f: impl FnOnce(&mut Character) -> T) -> T {
    with_save_lock(|| {
        let mut character = load_character();
        let out = f(&mut character);
        save_character(&character);
        out
    })
}

/// Category selects the ability. The modifier comes from the character sheet.
pub fn category_to_check(category: &Option<String>, character: &Character) -> (&'static str, i8) {
    match category.as_deref().unwrap_or("").to_lowercase().as_str() {
        "physical" => ("Strength", character.str_mod),
        "mental" => ("Intelligence", character.int_mod),
        "social" => ("Charisma", character.cha_mod),
        "creative" => ("Wisdom", character.wis_mod),
        _ => ("general", 0),
    }
}

pub fn item_effect(item: &str) -> &'static str {
    let lower = item.to_lowercase();
    if lower.contains("potion") || lower.contains("heal") {
        "healing"
    } else if lower.contains("torch") || lower.contains("lantern") || lower.contains("light") {
        "light"
    } else if lower.contains("tool") || lower.contains("rope") || lower.contains("kit") {
        "tool"
    } else if lower.contains("sword") || lower.contains("weapon") || lower.contains("dagger") {
        "weapon"
    } else {
        "mundane"
    }
}

pub fn unlock_achievement(campaign: &mut Campaign, name: &str) {
    if !campaign.achievements.contains(&name.to_string()) {
        campaign.achievements.push(name.to_string());
        println!("Achievement unlocked: {name}");
    }
}

pub fn random_encounter(module_id: Option<&str>) -> &'static str {
    match module_id {
        Some("delian") => "A pair of goblin scouts ambush you from the shadows!",
        Some("death-house") => "The walls groan and a pale, restless spirit drifts into the room...",
        Some("wild-sheep") => {
            "A confused, half-polymorphed animal (part sheep, part something else) blocks the path."
        }
        _ => "Something stirs in the darkness nearby...",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_mods_come_from_the_sheet() {
        let mut character = fresh_character();
        character.int_mod = 3;
        character.cha_mod = -1;
        assert_eq!(category_to_check(&Some("mental".into()), &character).1, 3);
        assert_eq!(
            category_to_check(&Some("social".into()), &character).0,
            "Charisma"
        );
        assert_eq!(category_to_check(&None, &character), ("general", 0));
    }

    #[test]
    fn item_effects_match_the_name() {
        assert_eq!(item_effect("Healing potion"), "healing");
        assert_eq!(item_effect("Torch"), "light");
        assert_eq!(item_effect("Rope (50 ft)"), "tool");
        assert_eq!(item_effect("Rusty dagger"), "weapon");
        assert_eq!(item_effect("Letter"), "mundane");
    }
}
