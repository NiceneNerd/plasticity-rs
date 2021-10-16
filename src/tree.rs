use eframe::egui::{self, CollapsingHeader, Ui};

#[derive(Debug, Default, Clone)]
pub struct Tree(pub(crate) String, pub(crate) usize, pub(crate) Vec<Tree>);

impl Tree {
    pub fn ui(&mut self, ui: &mut Ui, selected_index: &mut usize) {
        self.child_ui(ui, 0, selected_index)
    }

    fn child_ui(&mut self, ui: &mut Ui, depth: usize, selected_index: &mut usize) {
        if !self.2.is_empty() {
            let response = CollapsingHeader::new(&self.0)
                .default_open(depth < 1)
                .selectable(true)
                .selected(*selected_index == self.1)
                .show(ui, |ui| self.children_ui(ui, depth + 1, selected_index));
            if response.header_response.clicked() {
                *selected_index = self.1;
            }
        } else if ui
            .selectable_label(*selected_index == self.1, &self.0)
            .clicked()
        {
            *selected_index = self.1;
        }
    }

    fn children_ui(&mut self, ui: &mut Ui, depth: usize, selected_index: &mut usize) {
        self.2.iter_mut().for_each(|tree| {
            tree.child_ui(ui, depth, selected_index);
        });
    }
}
