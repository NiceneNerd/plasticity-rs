use std::{
    borrow::Cow,
    sync::mpsc::{channel, Receiver, Sender},
};

use anyhow::{Context, Error, Result};
use eframe::{
    egui::{self, menu, Color32, FontDefinitions, Frame, Vec2},
    epi,
};
use roead::aamp::ParamList;

use crate::{program::AIProgram, tree::Tree};

#[derive(Debug, Clone, Copy)]
enum Tab {
    AI,
    Action,
    Behaviour,
    Query,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    AIProgram(AIProgram),
    Tree(Vec<Tree>),
    Null,
}

pub struct App {
    aiprog: Option<AIProgram>,
    tree: Vec<Tree>,
    selected_ai: usize,
    tab: Tab,
    messengers: (Sender<Result<Message>>, Receiver<Result<Message>>),
    show_error: bool,
    error: Option<String>,
    show_busy: bool,
}

impl Default for App {
    fn default() -> Self {
        App {
            aiprog: None,
            tree: vec![],
            selected_ai: 0,
            tab: Tab::AI,
            messengers: channel(),
            show_error: false,
            error: None,
            show_busy: false,
        }
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "Plasticity"
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
                .family_and_size
                .iter_mut()
                .for_each(|(_, (_, size))| {
                    *size *= 1.25;
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
        let send = self.messengers.0.clone();
        let task = Box::new(task);
        std::thread::spawn(move || {
            send.send(task());
        });
    }

    fn handle_events(&mut self) {
        if let Ok(res) = self.messengers.1.try_recv() {
            self.show_busy = false;
            match res {
                Ok(msg) => match msg {
                    Message::AIProgram(aiprog) => {
                        self.aiprog = Some(aiprog.clone());
                        self.start_task(move || aiprog.to_tree().map(|t| Message::Tree(t)));
                    }
                    Message::Tree(tree) => self.tree = tree,
                    _ => (),
                },
                Err(e) => self.show_error(e),
            }
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
                            self.start_task(move || {
                                AIProgram::new(&file).map(|a| Message::AIProgram(a))
                            });
                        }
                    }
                    if ui.button("Save").clicked() {
                        println!("Save");
                    }
                    if ui.button("Save As").clicked() {
                        println!("Save As");
                    }
                    if ui.button("Exit").clicked() {
                        println!("Exit");
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
        egui::CentralPanel::default().show(ctx, |_ui| {
            egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(matches!(self.tab, Tab::AI), "AIs")
                        .clicked()
                    {
                        self.tab = Tab::AI;
                    };
                    if ui
                        .selectable_label(matches!(self.tab, Tab::Action), "Actions")
                        .clicked()
                    {
                        self.tab = Tab::Action;
                    };
                    if ui
                        .selectable_label(matches!(self.tab, Tab::Behaviour), "Behaviours")
                        .clicked()
                    {
                        self.tab = Tab::Behaviour;
                    };
                    if ui
                        .selectable_label(matches!(self.tab, Tab::Query), "Queries")
                        .clicked()
                    {
                        self.tab = Tab::Query;
                    };
                })
            });
            egui::CentralPanel::default().show(ctx, |ui| match self.tab {
                Tab::AI => ui.label("AI stuff"),
                Tab::Action => ui.label("Action stuff"),
                Tab::Behaviour => ui.label("Behaviour stuff"),
                Tab::Query => ui.label("Query stuff"),
            });
        });
    }

    fn render_error(&mut self, ctx: &egui::CtxRef) {
        let mut show = self.show_error;
        if self.show_error {
            egui::Window::new("Error").open(&mut show).show(ctx, |ui| {
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
