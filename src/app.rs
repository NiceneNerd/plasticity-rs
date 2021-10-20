use crate::{program::AIProgram, tree::Tree, util::*};
use anyhow::{Error, Result};
use eframe::{
    egui::{self, menu, FontDefinitions, Frame, Ui, Vec2},
    epi,
};
use roead::aamp::{hash_name, ParamList, Parameter};
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    AI,
    Action,
    Behaviour,
    Query,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::AI => f.write_fmt(format_args!("AI")),
            Self::Action => f.write_fmt(format_args!("Action")),
            Self::Behaviour => f.write_fmt(format_args!("Behavior")),
            Self::Query => f.write_fmt(format_args!("Query")),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    AIProgram(AIProgram),
    Tree(Vec<Tree>),
    Null,
}

pub struct App {
    file: Option<PathBuf>,
    aiprog: Option<AIProgram>,
    init_prog: Option<AIProgram>,
    tree: Vec<Tree>,
    cache: HashMap<&'static str, Vec<String>>,
    selected_ai: usize,
    last_selected: HashMap<Category, usize>,
    tab: Category,
    messengers: (Sender<Result<Message>>, Receiver<Result<Message>>),
    show_error: bool,
    error: Option<String>,
    show_busy: bool,
    title: String,
}

impl Default for App {
    fn default() -> Self {
        App {
            file: None,
            aiprog: None,
            init_prog: None,
            tree: vec![],
            cache: HashMap::with_capacity(3),
            selected_ai: 0,
            last_selected: HashMap::with_capacity(4),
            tab: Category::AI,
            messengers: channel(),
            show_error: false,
            error: None,
            show_busy: false,
            title: "Plasticity".into(),
        }
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        self.title.as_str()
    }

    fn setup(
        &mut self,
        ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        ctx.set_fonts({
            let mut font_defs = FontDefinitions::default();
            font_defs.font_data.insert(
                "Roboto".to_owned(),
                Cow::Borrowed(include_bytes!("../data/Roboto.ttf")),
            );
            font_defs.font_data.insert(
                "NotoSansJP".to_owned(),
                Cow::Borrowed(include_bytes!("../data/NotoSansJP.otf")),
            );
            font_defs.font_data.insert(
                "SourceCodePro".to_owned(),
                Cow::Borrowed(include_bytes!("../data/SourceCodePro.ttf")),
            );
            font_defs
                .fonts_for_family
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "Roboto".to_owned());
            font_defs
                .fonts_for_family
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(1, "NotoSansJP".to_owned());
            font_defs
                .fonts_for_family
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(1, "SourceCodePro".to_owned());
            font_defs
                .family_and_size
                .iter_mut()
                .for_each(|(_, (_, size))| {
                    *size *= 1.2;
                });
            font_defs
        })
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        self.render_menu(ctx);
        self.render_side_panel(ctx);
        self.render_main(ctx);
        self.render_error(ctx);
        self.render_busy(ctx);
        self.handle_events();
    }
}

impl App {
    #[allow(unused_must_use)]
    fn start_task<F: Fn() -> Result<Message> + Send + 'static>(&mut self, task: F) {
        self.show_busy = true;
        let sender = self.messengers.0.clone();
        let task = Box::new(task);
        std::thread::spawn(move || {
            sender.send(task());
        });
    }

    fn init_hashes(&mut self) {
        if let Some(aiprog) = self.aiprog.as_ref() {
            self.cache.insert(
                "child_names",
                (0..aiprog.behaviors_offset())
                    .into_iter()
                    .map(|i| aiprog.entry_name_from_index(i).unwrap().to_owned())
                    .collect(),
            );
            self.cache.insert(
                "behaviour_names",
                aiprog
                    .behaviors()
                    .into_iter()
                    .map(|ai| AIProgram::entry_name(ai).unwrap().to_owned())
                    .collect(),
            );
            self.cache.insert(
                "group_names",
                [String::new()]
                    .into_iter()
                    .chain(aiprog.ais().into_iter().map(|ai| {
                        ai.objects()["Def"]
                            .params()
                            .get(&hash_name("Name"))
                            .unwrap()
                            .as_string()
                            .unwrap()
                            .to_string()
                    }))
                    .collect::<BTreeSet<String>>()
                    .into_iter()
                    .collect(),
            );
        }
    }

    fn handle_events(&mut self) {
        if let Ok(res) = self.messengers.1.try_recv() {
            self.show_busy = false;
            match res {
                Ok(msg) => match msg {
                    Message::AIProgram(aiprog) => {
                        self.selected_ai = 0;
                        self.last_selected = HashMap::with_capacity(4);
                        self.init_prog = Some(aiprog.clone());
                        self.aiprog = Some(aiprog.clone());
                        self.init_hashes();
                        self.start_task(move || aiprog.to_tree().map(Message::Tree));
                    }
                    Message::Tree(tree) => self.tree = tree,
                    _ => (),
                },
                Err(e) => self.show_error(e),
            }
        }
        if *self.last_selected.get(&self.tab).unwrap_or(&0) != self.selected_ai
            && self.aiprog.is_some()
        {
            let aiprog = self.aiprog.as_ref().unwrap();
            self.tab = if self.selected_ai < aiprog.actions_offset() {
                Category::AI
            } else if self.selected_ai < aiprog.behaviors_offset() {
                Category::Action
            } else if self.selected_ai < aiprog.queries_offset() {
                Category::Behaviour
            } else {
                Category::Query
            };
            self.last_selected.insert(self.tab, self.selected_ai);
        }
        if self.init_prog != self.aiprog && !self.title.starts_with('*') {
            self.title = format!("*{}", self.title);
        }
    }

    fn render_menu(&mut self, ctx: &egui::CtxRef) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu(ui, "File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Some(file) = rfd::FileDialog::new()
                            .add_filter("BOTW AI Program", &["baiprog", "yml"])
                            .pick_file()
                        {
                            self.title = format!(
                                "{} - Plasticity",
                                file.file_name().unwrap().to_string_lossy()
                            );
                            self.file = Some(file.clone());
                            self.start_task(move || AIProgram::new(&file).map(Message::AIProgram));
                        }
                    }
                    if ui.button("Save").clicked() && self.aiprog.is_some() && self.file.is_some() {
                        let file = self.file.clone().unwrap();
                        let aiprog = self.aiprog.clone().unwrap();
                        self.start_task(move || {
                            std::fs::write(&file, &aiprog.save())
                                .map(|_| Message::Null)
                                .map_err(|e| e.into())
                        })
                    }
                    if ui.button("Save As").clicked() {
                        if let Some(file) = rfd::FileDialog::new()
                            .add_filter("BOTW AI Program", &["baiprog", "yml"])
                            .save_file()
                        {
                            let aiprog = self.aiprog.clone().unwrap();
                            self.start_task(move || {
                                std::fs::write(&file, &aiprog.save())
                                    .map(|_| Message::Null)
                                    .map_err(|e| e.into())
                            })
                        }
                    }
                    if ui.button("Exit").clicked() {
                        std::process::exit(0);
                    }
                });
            });
        });
    }

    fn render_side_panel(&mut self, ctx: &egui::CtxRef) {
        egui::SidePanel::left("tree_panel")
            .max_width(200.0)
            .resizable(true)
            .frame(Frame {
                margin: Vec2::new(8.0, 2.0),
                corner_radius: 0.0,
                fill: ctx.style().visuals.extreme_bg_color,
                stroke: ctx.style().visuals.window_stroke(),
                ..Default::default()
            })
            .show(ctx, |ui| {
                egui::ScrollArea::auto_sized().show(ui, |ui| {
                    self.tree
                        .iter_mut()
                        .for_each(|t| t.ui(ui, &mut self.selected_ai));
                });
            });
    }

    fn render_main(&mut self, ctx: &egui::CtxRef) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(aiprog) = self.aiprog.as_ref() {
                let actions_offset = aiprog.actions_offset();
                let behaviors_offset = aiprog.behaviors_offset();
                let queries_offset = aiprog.queries_offset();
                let show_ais = !aiprog.ais().is_empty();
                let show_actions = !aiprog.actions().is_empty();
                let show_behaviours = !aiprog.behaviors().is_empty();
                let show_queries = !aiprog.queries().is_empty();
                egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if show_ais
                            && ui
                                .selectable_label(matches!(self.tab, Category::AI), "AIs")
                                .clicked()
                        {
                            self.tab = Category::AI;
                            self.selected_ai = *self.last_selected.get(&Category::AI).unwrap_or(&0);
                        }
                        if show_actions
                            && ui
                                .selectable_label(matches!(self.tab, Category::Action), "Actions")
                                .clicked()
                        {
                            self.tab = Category::Action;
                            self.selected_ai = self
                                .last_selected
                                .get(&Category::Action)
                                .copied()
                                .unwrap_or(actions_offset)
                        }
                        if show_behaviours
                            && ui
                                .selectable_label(
                                    matches!(self.tab, Category::Behaviour),
                                    "Behaviours",
                                )
                                .clicked()
                        {
                            self.tab = Category::Behaviour;
                            self.selected_ai = self
                                .last_selected
                                .get(&Category::Behaviour)
                                .copied()
                                .unwrap_or(behaviors_offset)
                        }
                        if show_queries
                            && ui
                                .selectable_label(matches!(self.tab, Category::Query), "Queries")
                                .clicked()
                        {
                            self.tab = Category::Query;
                            self.selected_ai = self
                                .last_selected
                                .get(&Category::Query)
                                .copied()
                                .unwrap_or(queries_offset)
                        }
                    })
                });
                self.render_editor(ui, ctx);
            }
        });
    }

    fn render_editor(&mut self, ui: &mut Ui, ctx: &egui::CtxRef) {
        let mut update_tree = false;
        egui::ScrollArea::auto_sized().show(ui, |ui| {
            if self.aiprog.is_some() {
                ui.horizontal(|_ui| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::auto_sized()
                            .id_source("editor")
                            .show(ui, |ui| {
                                if let Some(aiprog) = self.aiprog.as_mut() {
                                    egui::ComboBox::from_label("Current Entry")
                                        .width(ui.available_width() - 125.0)
                                        .selected_text(
                                            aiprog.entry_name_from_index(self.selected_ai).unwrap(),
                                        )
                                        .show_ui(ui, |ui| {
                                            (match self.tab {
                                                Category::AI => aiprog.ais(),
                                                Category::Action => aiprog.actions(),
                                                Category::Behaviour => aiprog.behaviors(),
                                                Category::Query => aiprog.queries(),
                                            })
                                            .into_iter()
                                            .enumerate()
                                            .for_each(|(i, list)| {
                                                let idx = i + match self.tab {
                                                    Category::AI => 0,
                                                    Category::Action => aiprog.actions_offset(),
                                                    Category::Behaviour => {
                                                        aiprog.behaviors_offset()
                                                    }
                                                    Category::Query => aiprog.queries_offset(),
                                                };
                                                ui.selectable_value(
                                                    &mut self.selected_ai,
                                                    idx,
                                                    format!(
                                                        "{}_{}. {}",
                                                        self.tab,
                                                        i,
                                                        AIProgram::entry_name(list).unwrap()
                                                    ),
                                                );
                                            });
                                        });
                                }
                                update_tree = update_tree || self.render_definition(ui);
                                update_tree = update_tree || self.render_ai_children(ui);
                                self.render_sinst_parameters(ui);
                                self.render_behaviour_indexes(ui);
                            });
                    });
                });
            };
        });
        if update_tree {
            let aiprog = self.aiprog.clone().unwrap();
            self.start_task(move || aiprog.to_tree().map(Message::Tree));
        }
    }

    fn render_definition(&mut self, ui: &mut Ui) -> bool {
        let mut update_tree = false;
        if let Some(aiprog) = self.aiprog.as_mut() {
            let ai = aiprog.item_mut_at_index(self.selected_ai);
            if let Some(defs) = ai.objects_mut().get_mut(hash_name("Def")) {
                egui::CollapsingHeader::new("Definition")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("def").num_columns(2).show(ui, |ui| {
                            if let Some(name) = defs
                                .params_mut()
                                .get_mut(&hash_name("Name"))
                                .and_then(|n| match n {
                                    Parameter::StringRef(s) => Some(s),
                                    _ => None,
                                })
                            {
                                ui.label("Name");
                                if ui.text_edit_singleline(name).changed() {
                                    update_tree = true;
                                };
                                ui.end_row();
                            };
                            if let Some(name) = defs
                                .params_mut()
                                .get_mut(&hash_name("ClassName"))
                                .and_then(|n| match n {
                                    Parameter::String32(s) => Some(s),
                                    _ => None,
                                })
                            {
                                ui.label("ClassName");
                                // ui.text_edit_singleline(name);
                                egui::ComboBox::from_id_source("class_name")
                                    .selected_text(name.clone())
                                    .width(ui.spacing().text_edit_width)
                                    .show_ui(ui, |ui| {
                                        AIDEFS.classes(self.tab).for_each(|class| {
                                            ui.selectable_value(name, class.to_owned(), class);
                                        });
                                    });
                                ui.end_row();
                            };
                            if let Some(name) = defs
                                .params_mut()
                                .get_mut(&hash_name("GroupName"))
                                .and_then(|n| match n {
                                    Parameter::StringRef(s) => Some(s),
                                    _ => None,
                                })
                            {
                                ui.label("GroupName");
                                // ui.text_edit_singleline(name);
                                egui::ComboBox::from_id_source("group_name")
                                    .selected_text(name.clone())
                                    .width(ui.spacing().text_edit_width)
                                    .show_ui(ui, |ui| {
                                        let group_names = &self.cache["group_names"];
                                        group_names.iter().for_each(|ai_name| {
                                            ui.selectable_value(
                                                name,
                                                ai_name.clone(),
                                                JPEN_MAP
                                                    .get(ai_name.as_str())
                                                    .map(|s| format!("{} ({})", s, &ai_name))
                                                    .unwrap_or_else(|| ai_name.to_string()),
                                            );
                                        });
                                    });
                                ui.end_row();
                            };
                        });
                    });
            }
        }
        update_tree
    }

    fn render_ai_children(&mut self, ui: &mut Ui) -> bool {
        let mut update_tree = false;
        if self.aiprog.is_some() {
            let mut updates: HashMap<usize, String> = HashMap::new();
            let aiprog = self.aiprog.as_mut().unwrap();
            let ai_name = aiprog
                .item_at_index(self.selected_ai)
                .objects()
                .get(hash_name("Def"))
                .unwrap()
                .params()
                .get(&hash_name("Name"))
                .unwrap()
                .as_string()
                .unwrap()
                .to_string();
            let ai_count = aiprog.actions_offset();
            if aiprog
                .item_at_index(self.selected_ai)
                .objects()
                .get(hash_name("ChildIdx"))
                .is_some()
            {
                egui::CollapsingHeader::new("Children")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("child_idx").num_columns(2).show(ui, |ui| {
                            for (k, v) in aiprog
                                .item_mut_at_index(self.selected_ai)
                                .objects_mut()
                                .get_mut(hash_name("ChildIdx"))
                                .unwrap()
                                .params_mut()
                                .iter_mut()
                                .map(|(k, v)| (k, v.as_mut_int().unwrap()))
                            {
                                let child_name = try_name(*k);
                                ui.label(
                                    JPEN_MAP
                                        .get(&child_name.as_str())
                                        .unwrap_or(&child_name.as_str())
                                        .to_owned(),
                                );
                                let names = &self.cache["child_names"];
                                egui::ComboBox::from_id_source(k)
                                    .selected_text(names[(*v as usize)].clone())
                                    .width(ui.spacing().text_edit_width)
                                    .show_ui(ui, |ui| {
                                        names.iter().enumerate().for_each(|(i, name)| {
                                            let value = ui.selectable_value(v, i as i32, {
                                                if i < ai_count {
                                                    format!("AI_{}. {}", i, name)
                                                } else {
                                                    format!("Action_{}. {}", i - ai_count, name)
                                                }
                                            });
                                            if value.changed() {
                                                update_tree = true;
                                                if value.clicked() {
                                                    updates.insert(i, child_name.clone());
                                                }
                                            }
                                        });
                                    });
                                ui.end_row();
                            }
                        });
                    });
            }
            updates.into_iter().for_each(|(i, s)| {
                let defs = aiprog
                    .item_mut_at_index(i)
                    .objects_mut()
                    .get_mut(hash_name("Def"))
                    .unwrap()
                    .params_mut();
                defs.insert(hash_name("Name"), Parameter::StringRef(s));
                defs.insert(
                    hash_name("GroupName"),
                    Parameter::StringRef(ai_name.clone()),
                );
            });
            if update_tree {
                self.cache.insert(
                    "child_names",
                    (0..aiprog.behaviors_offset())
                        .into_iter()
                        .map(|i| aiprog.entry_name_from_index(i).unwrap().to_owned())
                        .collect(),
                );
            }
        }
        update_tree
    }

    fn render_sinst_parameters(&mut self, ui: &mut Ui) {
        if let Some(aiprog) = self.aiprog.as_mut() {
            let ai = aiprog.item_mut_at_index(self.selected_ai);
            if let Some(params) = ai.objects_mut().get_mut(hash_name("SInst")) {
                egui::CollapsingHeader::new("Static Instance Parameters")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("sinst").num_columns(2).show(ui, |ui| {
                            for (k, v) in params.params_mut().iter_mut() {
                                ui.label(try_name(*k));
                                Self::render_parameter(ui, v);
                                ui.end_row();
                            }
                        });
                    });
            }
        }
    }

    fn render_parameter(ui: &mut Ui, param: &mut Parameter) {
        match param {
            Parameter::Bool(b) => {
                ui.checkbox(b, "");
            }
            Parameter::Color(c) => {
                ui.horizontal(|ui| {
                    ui.label("A");
                    ui.add(egui::DragValue::new(&mut c.a).speed(0.1));
                    ui.label("R");
                    ui.add(egui::DragValue::new(&mut c.r).speed(0.1));
                    ui.label("G");
                    ui.add(egui::DragValue::new(&mut c.g).speed(0.1));
                    ui.label("B");
                    ui.add(egui::DragValue::new(&mut c.b).speed(0.1));
                });
            }
            Parameter::F32(f) => {
                ui.add(egui::DragValue::new(f).speed(0.1));
            }
            Parameter::Int(i) => {
                ui.add(egui::DragValue::new(i).speed(1));
            }
            Parameter::Quat(q) => {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut q.a).speed(0.1));
                    ui.add(egui::DragValue::new(&mut q.b).speed(0.1));
                    ui.add(egui::DragValue::new(&mut q.c).speed(0.1));
                    ui.add(egui::DragValue::new(&mut q.d).speed(0.1));
                });
            }
            Parameter::String256(s)
            | Parameter::String32(s)
            | Parameter::String64(s)
            | Parameter::StringRef(s) => {
                ui.text_edit_singleline(s);
            }
            Parameter::U32(u) => {
                ui.add(egui::DragValue::new(u).speed(1).clamp_range(0..=usize::MAX));
            }
            Parameter::Vec2(v) => {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut v.x).speed(0.1));
                    ui.add(egui::DragValue::new(&mut v.y).speed(0.1));
                });
            }
            Parameter::Vec3(v) => {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut v.x).speed(0.1));
                    ui.add(egui::DragValue::new(&mut v.y).speed(0.1));
                    ui.add(egui::DragValue::new(&mut v.z).speed(0.1));
                });
            }
            Parameter::Vec4(v) => {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut v.x).speed(0.1));
                    ui.add(egui::DragValue::new(&mut v.y).speed(0.1));
                    ui.add(egui::DragValue::new(&mut v.z).speed(0.1));
                    ui.add(egui::DragValue::new(&mut v.t).speed(0.1));
                });
            }
            _ => (),
        }
    }

    fn render_behaviour_indexes(&mut self, ui: &mut Ui) {
        if let Some(aiprog) = self.aiprog.as_mut() {
            let ai = aiprog.item_mut_at_index(self.selected_ai);
            if let Some(behaviours) = ai.objects_mut().get_mut(hash_name("BehaviorIdx")) {
                egui::CollapsingHeader::new("Behaviour Indexes")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("behaviour_idxs")
                            .num_columns(2)
                            .show(ui, |ui| {
                                let names = &self.cache["behaviour_names"];
                                behaviours.params_mut().iter_mut().for_each(|(k, v)| {
                                    let idx = v.as_mut_int().unwrap();
                                    ui.label(try_name(*k));
                                    egui::ComboBox::from_id_source(k)
                                        .width(ui.spacing().text_edit_width)
                                        .selected_text(names[(*idx) as usize].clone())
                                        .show_ui(ui, |ui| {
                                            names.iter().enumerate().for_each(|(i, name)| {
                                                ui.selectable_value(
                                                    idx,
                                                    i as i32,
                                                    format!("Behavior_{}. {}", i, name),
                                                );
                                            });
                                        });
                                    ui.end_row();
                                });
                            });
                    });
            }
        }
    }

    fn render_error(&mut self, ctx: &egui::CtxRef) {
        let mut show = self.show_error;
        if self.show_error {
            egui::Window::new("Error")
                .open(&mut show)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(self.error.as_ref().unwrap());
                    if ui.button("OK").clicked() {
                        self.show_error = false;
                    }
                });
            if !show {
                self.error = None;
            }
        }
    }

    fn render_busy(&mut self, ctx: &egui::CtxRef) {
        if self.show_busy {
            egui::Window::new("Plz Wait")
                .default_width(200.0)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.add(egui::widgets::ProgressBar::new(0.99).animate(true));
                });
        }
    }

    fn show_error(&mut self, error: Error) {
        self.show_error = true;
        self.error = Some(error.to_string());
    }
}
