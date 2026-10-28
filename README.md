# Todo-DnD

A Rust CLI application that combines a to-do list with a solo Dungeons & Dragons adventure. Complete real-world tasks to unlock dice rolls, and an AI Dungeon Master narrates your campaign based on the outcome. Optionally generates handmade pen-and-ink illustrations of key scenes.

## Features

- Add, list, and complete tasks
- Completing a task rolls a die (default d20) and triggers an AI-generated narrative
- Quest chains (multi-step tasks)
- Optional companion NPC
- Rest / recovery command
- Module-aware random encounters on low rolls
- Achievement / milestone log
- Richer character sheet (background, feature, goal)
- Inventory item effects (healing, light, tool, weapon)
- Better scene memory (recent summaries)
- Pen-and-ink campaign map of explored areas (with hints of the unknown)
- Inventory management and item use
- Simple HP tracking with automatic consequences on extreme rolls
- Module beat advancement (structured progress through free campaigns)
- Player journal notes
- Dice roll history
- Richer offline fallback narrative
- Task categories (physical/mental/social/creative) that map to ability checks
- Lightweight character sheet (name, class, level, HP, ability mods, inventory)
- Campaign progress notes and structured module support
- Gallery command for viewing saved illustrations
- Markdown session log export
- Optional pen-and-ink style illustrations (saved to `images/`) of key moments
- Built-in free campaign modules (Wild Sheep Chase, Delian Tomb, Death House)
- Persistent storage for tasks (`todos.json`) and campaign history (`campaign.json`)
- Works offline with a simple fallback narrative if no API key is set
- Compatible with any OpenAI-compatible API (OpenAI, xAI Grok, local models, etc.)

## Setup

1. Install Rust: https://rustup.rs/
2. Clone or copy this project
3. (Optional) Set environment variables for the AI DM and image generation:

   ```bash
   export OPENAI_API_KEY="your-xai-api-key"
   export OPENAI_BASE_URL="https://api.x.ai/v1"
   export MODEL="grok-2"
   export IMAGE_MODEL="grok-imagine-image-2.0"
   ```

4. Build and run:

   ```bash
   cargo run -- add "Finish taxes" -c physical
   cargo run -- list
   cargo run -- character --name "Aelar" --class "Ranger" --level 3
   cargo run -- complete 1
   cargo run -- gallery
   cargo run -- export
   cargo run -- campaigns
   cargo run -- start delian
   cargo run -- status
   ```

## Usage

```
todo-dnd add <description> [-c physical|mental|social|creative]
todo-dnd list
todo-dnd complete <index> [-s 20] [--no-image]
todo-dnd character [--name NAME] [--class CLASS] [--level N]
todo-dnd note "Found a strange symbol on the door"
todo-dnd use "torch"
todo-dnd hp -3
todo-dnd rolls
todo-dnd quest "Rescue the blacksmith" "Talk to the mayor|Search the tomb|Defeat the goblin boss"
todo-dnd companion "Pip" "Loyal but sarcastic rogue"
todo-dnd rest
todo-dnd achievements
todo-dnd character --name "Aelar" --class "Ranger" --background "Outlander" --goal "Find the lost sword"
todo-dnd use "potion"
todo-dnd map
todo-dnd gallery
todo-dnd export [-f session-log.md]
todo-dnd campaigns
todo-dnd start <id>
todo-dnd status
todo-dnd clear [--all]
```

## How it works

When you complete a task, the app rolls a die and sends the task description + roll result to an LLM acting as the Dungeon Master. The DM weaves the real-world accomplishment into the fantasy narrative and advances the story.

If an API key is available, it then generates a handmade pen-and-ink drawing of a key moment from the narrative (black ink, textured paper, cross-hatching, imperfect hand-drawn lines) and saves it under `images/`. The path is recorded in your campaign history.

Without an API key, a basic placeholder story is used so you can still play.

Enjoy turning your productivity into an epic quest with illustrated memories!
