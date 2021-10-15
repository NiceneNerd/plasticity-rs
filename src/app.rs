use anyhow::{Context, Result};
use eframe::{
    egui::{self, menu, Color32, Frame, Vec2},
    epi,
};
use roead;

use crate::tree::Tree;

#[derive(Debug, Clone, Copy)]
enum Tab {
    AI,
    Action,
    Behaviour,
    Query,
}

pub struct App {
    tree: Vec<Tree>,
    selected_ai: String,
    tab: Tab,
}

impl Default for App {
    fn default() -> Self {
        App {
            tree: vec![Tree::test()],
            selected_ai: String::new(),
            tab: Tab::AI,
        }
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "Plasticity"
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        self.render_menu(ctx);
        self.render_side_panel(ctx);
        self.render_main(ctx);
    }
}

impl App {
    fn render_menu(&mut self, ctx: &egui::CtxRef) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu(ui, "File", |ui| {
                    if ui.button("Open").clicked() {
                        println!("Open");
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
            .width_range(70.0..=200.0)
            .frame(Frame {
                margin: Vec2::new(8.0, 2.0),
                corner_radius: 0.0,
                fill: ctx.style().visuals.extreme_bg_color,
                stroke: ctx.style().visuals.window_stroke(),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.tree
                    .iter_mut()
                    .for_each(|t| t.ui(ui, 0, &mut self.selected_ai));
            });
    }

    fn render_main(&mut self, ctx: &egui::CtxRef) {
        egui::CentralPanel::default().show(ctx, |ui| {
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
}
