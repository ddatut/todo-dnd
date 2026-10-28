use clap::{Parser, Subcommand};
use todo_dnd::play;
use todo_dnd::state::{self, Character};

#[derive(Parser)]
#[command(name = "todo-dnd")]
#[command(about = "A to-do list that unlocks D&D adventures. `tasks` opens a plain to-do window. `todo-dnd play` opens the campaign.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new task
    Add {
        /// The task description
        description: String,
        /// Optional category: physical, mental, social, creative (maps to ability checks)
        #[arg(short, long)]
        category: Option<String>,
    },
    /// List all tasks
    List,
    /// Complete a task by index (1-based) and roll for the adventure
    Complete {
        /// Index of the task to complete
        index: usize,
        /// Optional dice type (default d20). e.g. 20, 6, 100
        #[arg(short, long, default_value_t = 20)]
        sides: u32,
        /// Skip generating an illustration
        #[arg(long)]
        no_image: bool,
    },
    /// Show campaign status / recent events
    Status,
    /// List available free campaign modules
    Campaigns,
    /// Start a structured campaign module
    Start {
        /// Module id: wild-sheep, delian, or death-house
        module: String,
    },
    /// View saved pen-and-ink illustrations
    Gallery,
    /// Create or update your character sheet
    Character {
        /// Character name
        #[arg(long)]
        name: Option<String>,
        /// Class (e.g. Fighter, Wizard)
        #[arg(long)]
        class: Option<String>,
        /// Level
        #[arg(long)]
        level: Option<u8>,
        /// Background
        #[arg(long)]
        background: Option<String>,
        /// Notable feature
        #[arg(long)]
        feature: Option<String>,
        /// Current goal
        #[arg(long)]
        goal: Option<String>,
    },
    /// Export full session log to Markdown
    Export {
        /// Output filename (default: session-log.md)
        #[arg(short, long, default_value = "session-log.md")]
        file: String,
    },
    /// Add a freeform player note to the journal
    Note {
        /// The note text
        text: String,
    },
    /// Use an inventory item (removes it and applies its effect)
    Use {
        /// Item name (partial match)
        item: String,
    },
    /// Add an item to inventory
    Item {
        /// Item name
        name: String,
    },
    /// Manually adjust HP (positive to heal, negative for damage)
    Hp {
        /// Amount to change HP by
        amount: i32,
    },
    /// Show recent dice rolls
    Rolls,
    /// Generate or refresh the pen-and-ink campaign map
    Map,
    /// Add a multi-step quest
    Quest {
        /// Quest name / id
        name: String,
        /// Steps separated by |
        steps: String,
    },
    /// Set or update your companion
    Companion {
        /// Companion name
        name: String,
        /// Short personality
        personality: String,
    },
    /// Take a rest (restore HP, quiet scene)
    Rest,
    /// Show achievements
    Achievements,
    /// Clear all completed tasks (or all if --all)
    Clear {
        #[arg(long)]
        all: bool,
    },
    /// Open the plain to-do window
    Work,
    /// Open the campaign window: map, character, illustrations, and tasks
    Play,
}

#[tokio::main]
async fn main() {
    todo_dnd::load_local_env();
    let cli = Cli::parse();
    match cli.command {
        Commands::Work => {
            if let Err(err) = todo_dnd::gui::run_work() {
                eprintln!("{err}");
            }
        }
        Commands::Play => {
            if let Err(err) = todo_dnd::gui::run_campaign() {
                eprintln!("{err}");
            }
        }
        Commands::Add { description, category } => {
            play::add_task(description, category);
            println!("Task added! Complete it to unlock a roll in your adventure.");
        }
        Commands::List => list_tasks(),
        Commands::Complete {
            index,
            sides,
            no_image,
        } => complete_task(index, sides, no_image).await,
        Commands::Status => status(),
        Commands::Campaigns => {
            println!("Available free campaign modules:\n");
            println!("{}", state::module_list_text());
            println!("\nStart one with: todo-dnd start <id>");
        }
        Commands::Start { module } => match play::start_module(&module) {
            Ok(intro) => {
                println!("Started: {} ({})", intro.name, intro.level);
                println!("{}\n", intro.summary);
                println!("First beat: {}", intro.first_beat);
                println!(
                    "Complete tasks to progress through the adventure. The DM will keep the story on track."
                );
            }
            Err(err) => eprintln!("{err}"),
        },
        Commands::Gallery => gallery(),
        Commands::Character {
            name,
            class,
            level,
            background,
            feature,
            goal,
        } => {
            let character = state::modify_character(|character| {
                if let Some(n) = name {
                    character.name = n;
                }
                if let Some(c) = class {
                    character.class = c;
                }
                if let Some(l) = level {
                    character.level = l;
                }
                if let Some(b) = background {
                    character.background = b;
                }
                if let Some(f) = feature {
                    character.feature = f;
                }
                if let Some(g) = goal {
                    character.goal = g;
                }
                character.clone()
            });
            print_character(&character);
        }
        Commands::Export { file } => match play::export_log(&file) {
            Ok(()) => println!("Session log exported to {file}"),
            Err(err) => eprintln!("{err}"),
        },
        Commands::Note { text } => {
            play::add_note(text);
            println!("Note added to journal.");
        }
        Commands::Use { item } => match play::use_item(&item) {
            Ok(message) => println!("{message}"),
            Err(err) => println!("{err}"),
        },
        Commands::Item { name } => match play::add_item(&name) {
            Ok(()) => println!("Added {name} to inventory."),
            Err(err) => eprintln!("{err}"),
        },
        Commands::Hp { amount } => println!("{}", play::adjust_hp(amount)),
        Commands::Rolls => rolls(),
        Commands::Map => match play::draw_map().await {
            Ok(path) => println!("Campaign map saved: {path}"),
            Err(err) => println!("{err}"),
        },
        Commands::Quest { name, steps } => match play::add_quest(&name, &steps) {
            Ok(count) => println!("Quest '{name}' added with {count} steps."),
            Err(err) => println!("{err}"),
        },
        Commands::Companion { name, personality } => {
            play::set_companion(name.clone(), personality);
            println!("Companion {name} joins the adventure.");
        }
        Commands::Rest => {
            let (restored, hp, max_hp) = play::rest();
            println!("You rest and recover {restored} HP (now {hp}/{max_hp}).");
            println!("The night passes quietly...");
        }
        Commands::Achievements => {
            let campaign = state::load_campaign();
            if campaign.achievements.is_empty() {
                println!("No achievements yet. Keep adventuring!");
            } else {
                println!("Achievements:");
                for achievement in &campaign.achievements {
                    println!("  - {achievement}");
                }
            }
        }
        Commands::Clear { all } => {
            play::clear_tasks(all);
            if all {
                println!("All tasks cleared.");
            } else {
                println!("Completed tasks cleared.");
            }
        }
    }
}

fn list_tasks() {
    let tasks = state::load_tasks();
    if tasks.is_empty() {
        println!("No tasks yet. Add some to start your adventure!");
        return;
    }
    println!("Your quests (to-do list):");
    for (i, task) in tasks.iter().enumerate() {
        let status = if task.completed { "[x]" } else { "[ ]" };
        let cat = task.category.as_deref().unwrap_or("");
        let cat_str = if cat.is_empty() {
            String::new()
        } else {
            format!(" ({cat})")
        };
        let quest = task
            .quest_id
            .as_ref()
            .map(|id| format!(" {{{id}}}"))
            .unwrap_or_default();
        println!(
            "{status} {}. {}{cat_str}{quest}",
            i + 1,
            task.description
        );
    }
    let pending = tasks.iter().filter(|t| !t.completed).count();
    println!("\n{pending} pending. Complete them to roll and progress the campaign!");
}

async fn complete_task(index: usize, sides: u32, no_image: bool) {
    if index == 0 {
        eprintln!("Invalid task index. Use `list` to see them.");
        return;
    }
    let task = match play::mark_complete(index - 1) {
        Ok(task) => task,
        Err(err) => {
            if err.starts_with("Invalid") {
                eprintln!("{err}");
            } else {
                println!("{err}");
            }
            return;
        }
    };
    println!("Task completed: \"{}\"", task.description);
    let pending = play::begin_tale(task, sides);
    if pending.check_name != "general" {
        println!("Treated as a {} check.", pending.check_name);
        println!(
            "You roll a d{}... {} {:+} = {}",
            pending.sides, pending.roll, pending.modifier, pending.total
        );
    } else {
        println!("You roll a d{}... {}", pending.sides, pending.roll);
    }
    if let Some(encounter) = &pending.encounter {
        println!("Encounter: {encounter}");
    }
    if let Some(note) = &pending.hp_note {
        println!("{note}");
    }
    if let Some(beat) = &pending.progress {
        println!("Progress advanced to: {beat}");
    }
    if let Some(note) = &pending.loyalty_note {
        println!("{note}");
    }
    let quest = pending.quest_complete.clone();
    if !no_image {
        println!("Sketching a pen-and-ink illustration of the key moment...");
    }
    let result = play::finish_tale(pending, no_image).await;
    println!("\n=== Dungeon Master ===\n{}\n", result.narrative);
    if let Some(path) = &result.image_path {
        println!("Illustration saved: {path}");
    } else if result.image_missing {
        println!("(Could not generate illustration — check API key / IMAGE_MODEL)");
    }
    if let Some(qid) = quest {
        println!("Quest complete: {qid}!");
    }
    println!("Adventure progress saved. Complete more tasks to continue!");
}

fn status() {
    let campaign = state::load_campaign();
    if campaign.events.is_empty() && campaign.active_module.is_none() {
        println!("Your campaign hasn't started yet. Complete a task to begin!");
        return;
    }
    let character = state::load_character();
    println!("Campaign status:");
    println!(
        "Character: {} the level {} {} (HP {}/{})",
        character.name, character.level, character.class, character.hp, character.max_hp
    );
    if let Some(id) = &campaign.active_module {
        if let Some(module) = state::get_module(id) {
            println!("Active module: {} ({})", module.name, module.level);
        } else {
            println!("Active module: {id}");
        }
    }
    println!(
        "Current scene: {}",
        if campaign.current_scene.is_empty() {
            "The adventure begins..."
        } else {
            &campaign.current_scene
        }
    );
    if !campaign.progress_note.is_empty() {
        println!("Progress: {}", campaign.progress_note);
    }
    if let Some(companion) = &campaign.companion {
        println!(
            "Companion: {} — {} (loyalty {})",
            companion.name, companion.personality, companion.loyalty
        );
    }
    if !campaign.notes.is_empty() {
        println!("Recent notes: {}", campaign.notes.len());
    }
    if let Some(map) = &campaign.map_path {
        println!("Latest map: {map}");
    }
    if !campaign.events.is_empty() {
        println!("\nRecent events:");
        for (i, event) in campaign.events.iter().rev().take(5).enumerate() {
            println!("{}. {event}", i + 1);
        }
    }
    if !campaign.images.is_empty() {
        println!("\nIllustrations:");
        for img in campaign.images.iter().rev().take(5) {
            println!("  - {img}");
        }
    }
}

fn gallery() {
    let campaign = state::load_campaign();
    if campaign.images.is_empty() {
        println!("No illustrations yet. Complete tasks to generate pen-and-ink scenes.");
        return;
    }
    println!("Pen-and-ink illustrations:");
    for (i, img) in campaign.images.iter().enumerate() {
        println!("{}. {img}", i + 1);
    }
    println!("\nOpen the files in your image viewer, or run `todo-dnd play`.");
}

fn rolls() {
    let campaign = state::load_campaign();
    if campaign.dice_history.is_empty() {
        println!("No dice rolls recorded yet.");
        return;
    }
    println!("Recent dice rolls:");
    for entry in campaign.dice_history.iter().rev().take(10) {
        println!("  {entry}");
    }
}

fn print_character(character: &Character) {
    println!("Character sheet:");
    println!(
        "  {} the level {} {}",
        character.name, character.level, character.class
    );
    if !character.background.is_empty() {
        println!("  Background: {}", character.background);
    }
    if !character.feature.is_empty() {
        println!("  Feature: {}", character.feature);
    }
    if !character.goal.is_empty() {
        println!("  Goal: {}", character.goal);
    }
    println!("  HP: {}/{}", character.hp, character.max_hp);
    println!(
        "  Mods: STR {} DEX {} CON {} INT {} WIS {} CHA {}",
        character.str_mod,
        character.dex_mod,
        character.con_mod,
        character.int_mod,
        character.wis_mod,
        character.cha_mod
    );
    if !character.inventory.is_empty() {
        println!("  Inventory: {}", character.inventory.join(", "));
    }
}
