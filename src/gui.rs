use crate::play;
use crate::state::{self, Campaign, Character, Task};
use eframe::egui;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

pub fn run_work() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("To Do")
            .with_inner_size([380.0, 540.0])
            .with_min_inner_size([280.0, 360.0])
            .with_icon(plain_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "ToDo",
        options,
        Box::new(|cc| {
            style_plain(&cc.egui_ctx);
            Ok(Box::new(WorkApp::new()))
        }),
    )
}

pub fn run_campaign() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Adventure")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Adventure",
        options,
        Box::new(|cc| {
            style_campaign(&cc.egui_ctx);
            Ok(Box::new(CampaignApp::new()))
        }),
    )
}

fn plain_icon() -> egui::IconData {
    egui::IconData {
        rgba: vec![232, 232, 232, 255],
        width: 1,
        height: 1,
    }
}

fn style_plain(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = egui::Color32::from_rgb(248, 248, 248);
    visuals.window_fill = egui::Color32::from_rgb(248, 248, 248);
    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(15.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(20.0, egui::FontFamily::Proportional),
    );
    ctx.set_style(style);
}

fn style_campaign(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    let paper = egui::Color32::from_rgb(244, 239, 230);
    visuals.panel_fill = paper;
    visuals.window_fill = paper;
    ctx.set_visuals(visuals);
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, dirty: &mut bool) {
    ui.label(label);
    if ui.text_edit_singleline(value).changed() {
        *dirty = true;
    }
}

fn mod_field(ui: &mut egui::Ui, label: &str, value: &mut i8, dirty: &mut bool) {
    ui.label(label);
    let mut number = i32::from(*value);
    if ui
        .add(egui::DragValue::new(&mut number).range(-5..=10))
        .changed()
    {
        *value = number.clamp(-5, 10) as i8;
        *dirty = true;
    }
}

struct WorkApp {
    tasks: Vec<Task>,
    draft: String,
    note: String,
    tx: Sender<()>,
    rx: Receiver<()>,
}

impl WorkApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tasks: state::load_tasks(),
            draft: String::new(),
            note: String::new(),
            tx,
            rx,
        }
    }

    fn reload(&mut self) {
        self.tasks = state::with_save_lock(state::load_tasks);
    }

    fn add_current(&mut self) {
        let description = self.draft.trim().to_string();
        if description.is_empty() {
            return;
        }
        play::add_task(description, None);
        self.draft.clear();
        self.note = "Saved".to_string();
        self.reload();
    }

    fn finish_in_background(&self, task: Task) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            runtime().block_on(play::tell(task, 20, false));
            let _ = tx.send(());
        });
    }
}

impl eframe::App for WorkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(700));
        while self.rx.try_recv().is_ok() {
            self.reload();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("To Do");
            ui.add_space(6.0);

            let mut submit = false;
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    submit |= ui.button("Add").clicked();
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.draft)
                            .hint_text("Add a task")
                            .desired_width(f32::INFINITY),
                    );
                    submit |= response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if submit {
                        response.request_focus();
                    }
                });
            });
            if submit {
                self.add_current();
            }

            ui.add_space(8.0);
            let open: Vec<usize> = self
                .tasks
                .iter()
                .enumerate()
                .filter(|(_, task)| !task.completed)
                .map(|(index, _)| index)
                .collect();
            let done: Vec<usize> = self
                .tasks
                .iter()
                .enumerate()
                .filter(|(_, task)| task.completed)
                .map(|(index, _)| index)
                .collect();

            egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 36.0)
                .show(ui, |ui| {
                    if open.is_empty() && done.is_empty() {
                        ui.label("Nothing on the list.");
                    }
                    for index in open {
                        let description = self.tasks[index].description.clone();
                        let mut checked = false;
                        if ui.checkbox(&mut checked, &description).changed() && checked {
                            match play::mark_complete(index) {
                                Ok(task) => {
                                    self.note = "Saved".to_string();
                                    self.finish_in_background(task);
                                    self.reload();
                                }
                                Err(_) => self.note = "Couldn't update that task.".to_string(),
                            }
                        }
                    }
                    if !done.is_empty() {
                        ui.add_space(8.0);
                        egui::CollapsingHeader::new(format!("Completed ({})", done.len())).show(
                            ui,
                            |ui| {
                                for index in done {
                                    let description = self.tasks[index].description.clone();
                                    let mut checked = true;
                                    if ui.checkbox(&mut checked, &description).changed() && !checked
                                    {
                                        if play::reopen_task(index).is_ok() {
                                            self.note = "Saved".to_string();
                                            self.reload();
                                        }
                                    }
                                }
                            },
                        );
                    }
                });

            ui.separator();
            ui.horizontal(|ui| {
                let left = self.tasks.iter().filter(|task| !task.completed).count();
                ui.label(format!("{left} left"));
                if self.tasks.iter().any(|task| task.completed)
                    && ui.button("Clear completed").clicked()
                {
                    play::clear_tasks(false);
                    self.note = "Saved".to_string();
                    self.reload();
                }
                if !self.note.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(&self.note).weak());
                    });
                }
            });
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AtlasTab {
    Map,
    Scenes,
}

enum UiMsg {
    Story { narrative: String, note: Option<String> },
    Notice(String),
}

struct CampaignApp {
    tasks: Vec<Task>,
    campaign: Campaign,
    character: Character,
    name: String,
    class_name: String,
    level: u8,
    background: String,
    feature: String,
    goal: String,
    str_mod: i8,
    dex_mod: i8,
    con_mod: i8,
    int_mod: i8,
    wis_mod: i8,
    cha_mod: i8,
    companion_name: String,
    companion_personality: String,
    sheet_dirty: bool,
    draft: String,
    category: String,
    note_draft: String,
    item_draft: String,
    narrative: String,
    notice: String,
    module_choice: String,
    tab: AtlasTab,
    textures: HashMap<String, egui::TextureHandle>,
    tx: Sender<UiMsg>,
    rx: Receiver<UiMsg>,
    busy: bool,
}

impl CampaignApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            tasks: Vec::new(),
            campaign: Campaign::default(),
            character: Character::default(),
            name: String::new(),
            class_name: String::new(),
            level: 1,
            background: String::new(),
            feature: String::new(),
            goal: String::new(),
            str_mod: 0,
            dex_mod: 0,
            con_mod: 0,
            int_mod: 0,
            wis_mod: 0,
            cha_mod: 0,
            companion_name: String::new(),
            companion_personality: String::new(),
            sheet_dirty: false,
            draft: String::new(),
            category: String::new(),
            note_draft: String::new(),
            item_draft: String::new(),
            narrative: String::new(),
            notice: String::new(),
            module_choice: String::new(),
            tab: AtlasTab::Map,
            textures: HashMap::new(),
            tx,
            rx,
            busy: false,
        };
        app.reload();
        app
    }

    fn reload(&mut self) {
        let (tasks, campaign, character) = state::with_save_lock(|| {
            (
                state::load_tasks(),
                state::load_campaign(),
                state::load_character(),
            )
        });
        self.tasks = tasks;
        self.campaign = campaign;
        self.character = character;
        if self.module_choice.is_empty() {
            self.module_choice = self.campaign.active_module.clone().unwrap_or_default();
        }
        if !self.sheet_dirty {
            self.name = self.character.name.clone();
            self.class_name = self.character.class.clone();
            self.level = self.character.level.max(1);
            self.background = self.character.background.clone();
            self.feature = self.character.feature.clone();
            self.goal = self.character.goal.clone();
            self.str_mod = self.character.str_mod;
            self.dex_mod = self.character.dex_mod;
            self.con_mod = self.character.con_mod;
            self.int_mod = self.character.int_mod;
            self.wis_mod = self.character.wis_mod;
            self.cha_mod = self.character.cha_mod;
            if let Some(companion) = &self.campaign.companion {
                self.companion_name = companion.name.clone();
                self.companion_personality = companion.personality.clone();
            }
        }
        if !self.busy {
            if let Some(text) = &self.campaign.last_narrative {
                self.narrative = text.clone();
            }
        }
    }

    fn save_sheet(&mut self) {
        let mut sheet = self.character.clone();
        sheet.name = self.name.trim().to_string();
        sheet.class = self.class_name.trim().to_string();
        sheet.level = self.level.max(1);
        sheet.background = self.background.trim().to_string();
        sheet.feature = self.feature.trim().to_string();
        sheet.goal = self.goal.trim().to_string();
        sheet.str_mod = self.str_mod;
        sheet.dex_mod = self.dex_mod;
        sheet.con_mod = self.con_mod;
        sheet.int_mod = self.int_mod;
        sheet.wis_mod = self.wis_mod;
        sheet.cha_mod = self.cha_mod;
        play::save_sheet(&sheet, &self.companion_name, &self.companion_personality);
        self.sheet_dirty = false;
        self.notice = "Character sheet saved.".to_string();
        self.reload();
    }

    fn complete(&mut self, index: usize) {
        let task = match play::mark_complete(index) {
            Ok(task) => task,
            Err(err) => {
                self.notice = err;
                return;
            }
        };
        let pending = play::begin_tale(task, 20);
        let mut lines = Vec::new();
        if pending.check_name != "general" {
            lines.push(format!(
                "{} check: d{} {} {:+} = {}",
                pending.check_name, pending.sides, pending.roll, pending.modifier, pending.total
            ));
        } else {
            lines.push(format!("Roll: d{} = {}", pending.sides, pending.roll));
        }
        if let Some(encounter) = &pending.encounter {
            lines.push(format!("Encounter: {encounter}"));
        }
        if let Some(note) = &pending.hp_note {
            lines.push(note.clone());
        }
        if let Some(beat) = &pending.progress {
            lines.push(format!("Progress: {beat}"));
        }
        if let Some(note) = &pending.loyalty_note {
            lines.push(note.clone());
        }
        self.notice = lines.join("\n");
        self.busy = true;
        self.narrative = "The Dungeon Master is thinking...".to_string();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = runtime().block_on(play::finish_tale(pending, false));
            let note = if let Some(path) = &result.image_path {
                Some(format!("Illustration saved: {path}"))
            } else if result.image_missing {
                Some(
                    "Could not generate illustration — check API key / IMAGE_MODEL".to_string(),
                )
            } else {
                None
            };
            let _ = tx.send(UiMsg::Story {
                narrative: result.narrative,
                note,
            });
        });
        self.reload();
    }

    fn show_image(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, path: &str, max_width: f32) {
        let texture = if let Some(texture) = self.textures.get(path) {
            texture.clone()
        } else {
            let Some(texture) = load_texture(ctx, path) else {
                ui.label(path);
                return;
            };
            self.textures.insert(path.to_string(), texture.clone());
            texture
        };
        let size = texture.size_vec2();
        let width = max_width.min(ui.available_width()).max(40.0);
        let height = if size.x <= 0.0 {
            width
        } else {
            width * size.y / size.x
        };
        ui.add(
            egui::Image::from_texture(&texture).fit_to_exact_size(egui::vec2(width, height)),
        );
        ui.label(egui::RichText::new(path).weak().small());
    }
}

impl eframe::App for CampaignApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(700));
        if let Ok(message) = self.rx.try_recv() {
            match message {
                UiMsg::Story { narrative, note } => {
                    self.narrative = narrative;
                    if let Some(note) = note {
                        self.notice = if self.notice.is_empty() {
                            note
                        } else {
                            format!("{}\n{note}", self.notice)
                        };
                    }
                }
                UiMsg::Notice(note) => self.notice = note,
            }
            self.busy = false;
            self.reload();
        } else if !self.sheet_dirty && !self.busy {
            self.reload();
        }

        self.character_panel(ctx);
        self.atlas_panel(ctx);
        self.story_panel(ctx);
    }
}

impl CampaignApp {
    fn character_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("character")
            .resizable(true)
            .default_width(290.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Character");
                    ui.label(format!(
                        "HP {}/{}",
                        self.character.hp, self.character.max_hp
                    ));
                    if self.character.max_hp > 0 {
                        let fraction =
                            (self.character.hp as f32 / self.character.max_hp as f32).clamp(0.0, 1.0);
                        ui.add(egui::ProgressBar::new(fraction).show_percentage());
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Rest").clicked() {
                            let (restored, hp, max_hp) = play::rest();
                            self.notice =
                                format!("Rested. Recovered {restored} HP (now {hp}/{max_hp}).");
                            self.reload();
                        }
                        if ui.button("Save sheet").clicked() {
                            self.save_sheet();
                        }
                    });

                    field(ui, "Name", &mut self.name, &mut self.sheet_dirty);
                    field(ui, "Class", &mut self.class_name, &mut self.sheet_dirty);
                    ui.label("Level");
                    let mut level = i32::from(self.level);
                    if ui
                        .add(egui::DragValue::new(&mut level).range(1..=20))
                        .changed()
                    {
                        self.level = level.clamp(1, 20) as u8;
                        self.sheet_dirty = true;
                    }
                    field(ui, "Background", &mut self.background, &mut self.sheet_dirty);
                    field(ui, "Feature", &mut self.feature, &mut self.sheet_dirty);
                    field(ui, "Goal", &mut self.goal, &mut self.sheet_dirty);

                    ui.add_space(6.0);
                    ui.label("Ability modifiers");
                    egui::Grid::new("mods").num_columns(4).show(ui, |ui| {
                        mod_field(ui, "STR", &mut self.str_mod, &mut self.sheet_dirty);
                        mod_field(ui, "DEX", &mut self.dex_mod, &mut self.sheet_dirty);
                        ui.end_row();
                        mod_field(ui, "CON", &mut self.con_mod, &mut self.sheet_dirty);
                        mod_field(ui, "INT", &mut self.int_mod, &mut self.sheet_dirty);
                        ui.end_row();
                        mod_field(ui, "WIS", &mut self.wis_mod, &mut self.sheet_dirty);
                        mod_field(ui, "CHA", &mut self.cha_mod, &mut self.sheet_dirty);
                        ui.end_row();
                    });

                    ui.add_space(8.0);
                    ui.label("Companion");
                    field(ui, "Name", &mut self.companion_name, &mut self.sheet_dirty);
                    field(
                        ui,
                        "Personality",
                        &mut self.companion_personality,
                        &mut self.sheet_dirty,
                    );
                    if let Some(companion) = &self.campaign.companion {
                        ui.label(format!("Loyalty {}", companion.loyalty));
                    }

                    ui.add_space(8.0);
                    ui.label("Inventory");
                    let items = self.character.inventory.clone();
                    for item in &items {
                        ui.horizontal(|ui| {
                            ui.label(item);
                            if ui.button("Use").clicked() {
                                self.notice = match play::use_item(item) {
                                    Ok(message) => message,
                                    Err(err) => err,
                                };
                                self.reload();
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.item_draft)
                                .hint_text("Add item")
                                .desired_width(140.0),
                        );
                        if ui.button("Add").clicked() {
                            if play::add_item(&self.item_draft).is_ok() {
                                self.item_draft.clear();
                                self.reload();
                            }
                        }
                    });
                });
            });
    }

    fn atlas_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("atlas")
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, AtlasTab::Map, "Map");
                    ui.selectable_value(&mut self.tab, AtlasTab::Scenes, "Scenes");
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                    AtlasTab::Map => self.map_tab(ctx, ui),
                    AtlasTab::Scenes => self.scenes_tab(ctx, ui),
                });
            });
    }

    fn map_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let draw = ui
            .add_enabled(!self.busy, egui::Button::new("Draw map"))
            .clicked();
        if draw {
            self.busy = true;
            self.notice = "Sketching the map...".to_string();
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let message = match runtime().block_on(play::draw_map()) {
                    Ok(path) => format!("Map saved: {path}"),
                    Err(err) => err,
                };
                let _ = tx.send(UiMsg::Notice(message));
            });
        }
        ui.add_space(6.0);
        if let Some(path) = self.campaign.map_path.clone() {
            let width = ui.available_width();
            self.show_image(ctx, ui, &path, width);
        } else {
            ui.label("No map yet. Explored places still show up below.");
        }
        ui.add_space(8.0);
        ui.label("Explored");
        if self.campaign.explored_areas.is_empty() {
            ui.label("Nowhere yet.");
        }
        for area in &self.campaign.explored_areas.clone() {
            ui.label(format!("• {area}"));
        }
        if let Some(id) = &self.campaign.active_module.clone() {
            if let Some(module) = state::get_module(id) {
                let unknown = module
                    .beats
                    .len()
                    .saturating_sub(self.campaign.current_beat + 1);
                if unknown > 0 {
                    ui.label(
                        egui::RichText::new(format!("{unknown} uncharted stretch(es) remain"))
                            .weak(),
                    );
                }
            }
        }
    }

    fn scenes_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let images = self.campaign.images.clone();
        if images.is_empty() {
            ui.label("No scenes yet. Finish a task with an API key set and one will be drawn here.");
            return;
        }
        for path in images.iter().rev() {
            let width = ui.available_width();
            self.show_image(ctx, ui, path, width);
            ui.add_space(10.0);
        }
    }

    fn story_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Adventure");
                egui::ComboBox::from_id_salt("module")
                    .selected_text(if self.module_choice.is_empty() {
                        "No module".to_string()
                    } else {
                        self.module_choice.clone()
                    })
                    .show_ui(ui, |ui| {
                        for module in state::modules() {
                            ui.selectable_value(
                                &mut self.module_choice,
                                module.id.to_string(),
                                module.name,
                            );
                        }
                    });
                if ui.button("Start module").clicked() && !self.module_choice.is_empty() {
                    match play::start_module(&self.module_choice) {
                        Ok(intro) => {
                            self.notice = format!("Started {}. First beat: {}", intro.name, intro.first_beat);
                        }
                        Err(err) => self.notice = err,
                    }
                    self.reload();
                }
            });
            if let Some(id) = &self.campaign.active_module {
                if let Some(module) = state::get_module(id) {
                    let beat = module
                        .beats
                        .get(self.campaign.current_beat)
                        .copied()
                        .unwrap_or("the adventure");
                    ui.label(format!(
                        "{} — beat {}: {beat}",
                        module.name,
                        self.campaign.current_beat + 1
                    ));
                }
            }
            if !self.campaign.current_scene.is_empty() {
                ui.label(egui::RichText::new(&self.campaign.current_scene).italics());
            }

            if !self.notice.is_empty() {
                ui.add_space(4.0);
                ui.label(&self.notice);
            }

            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .id_salt("story")
                .max_height(220.0)
                .show(ui, |ui| {
                    if self.narrative.is_empty() {
                        ui.label("Complete a task to hear from the Dungeon Master.");
                    } else {
                        ui.label(&self.narrative);
                    }
                });

            ui.separator();
            ui.horizontal(|ui| {
                let add = ui.button("Add task").clicked();
                egui::ComboBox::from_id_salt("category")
                    .selected_text(if self.category.is_empty() {
                        "general"
                    } else {
                        self.category.as_str()
                    })
                    .show_ui(ui, |ui| {
                        for choice in ["", "physical", "mental", "social", "creative"] {
                            let label = if choice.is_empty() { "general" } else { choice };
                            ui.selectable_value(&mut self.category, choice.to_string(), label);
                        }
                    });
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.draft)
                        .hint_text("New task")
                        .desired_width(f32::INFINITY),
                );
                let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if add || enter {
                    let description = self.draft.trim().to_string();
                    if !description.is_empty() {
                        let category = if self.category.is_empty() {
                            None
                        } else {
                            Some(self.category.clone())
                        };
                        play::add_task(description, category);
                        self.draft.clear();
                        self.reload();
                    }
                }
            });

            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt("tasks")
                .max_height(180.0)
                .show(ui, |ui| {
                    let tasks = self.tasks.clone();
                    if tasks.is_empty() {
                        ui.label("No tasks yet.");
                    }
                    for (index, task) in tasks.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if task.completed {
                                ui.label(format!("[x] {}", task.description));
                                return;
                            }
                            if ui
                                .add_enabled(!self.busy, egui::Button::new("Done"))
                                .clicked()
                            {
                                self.complete(index);
                            }
                            let cat = task.category.as_deref().unwrap_or("");
                            let extra = if cat.is_empty() {
                                String::new()
                            } else {
                                format!(" ({cat})")
                            };
                            let quest = task
                                .quest_id
                                .as_ref()
                                .map(|id| format!(" — {id}"))
                                .unwrap_or_default();
                            ui.label(format!("{}{extra}{quest}", task.description));
                        });
                    }
                });

            ui.separator();
            ui.label("Journal");
            ui.horizontal(|ui| {
                let add = ui.button("Add note").clicked();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.note_draft)
                        .hint_text("A reminder or clue")
                        .desired_width(f32::INFINITY),
                );
                let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if add || enter {
                    let text = self.note_draft.trim().to_string();
                    if !text.is_empty() {
                        play::add_note(text);
                        self.note_draft.clear();
                        self.reload();
                    }
                }
            });
            for note in self.campaign.notes.iter().rev().take(6) {
                ui.label(format!("• {note}"));
            }
        });
    }
}

fn load_texture(ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?.into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    Some(ctx.load_texture(path, color, egui::TextureOptions::LINEAR))
}
