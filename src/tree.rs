use eframe::egui::{self, CollapsingHeader, Ui};

#[derive(Debug, Default, Clone)]
pub struct Tree(String, Vec<Tree>);

impl Tree {
    pub(crate) fn test() -> Self {
        Self(
            "AIRoot".to_owned(),
            vec![
                Tree(
                    "SomeAIThing".to_owned(),
                    vec![
                        Tree("AnAISubThing1".to_owned(), vec![]),
                        Tree("AnAISubThing2".to_owned(), vec![]),
                        Tree("AnAISubThing3".to_owned(), vec![]),
                        Tree("AnAISubThing4".to_owned(), vec![]),
                    ],
                ),
                Tree(
                    "SomeOtherAIThing".to_owned(),
                    vec![
                        Tree("AnAISubThing1a".to_owned(), vec![]),
                        Tree("AnAISubThing2a".to_owned(), vec![]),
                        Tree("AnAISubThing3a".to_owned(), vec![]),
                    ],
                ),
            ],
        )
    }

    pub fn ui(&mut self, ui: &mut Ui, depth: usize, selected_name: &mut String) {
        if !self.1.is_empty() {
            let response = CollapsingHeader::new(&self.0)
                .default_open(depth < 1)
                .selectable(true)
                .selected(selected_name.as_str() == self.0.as_str())
                .show(ui, |ui| self.children_ui(ui, depth + 1, selected_name));
            if response.header_response.clicked() {
                *selected_name = self.0.clone();
            }
        } else if ui
            .selectable_label(selected_name.as_str() == self.0.as_str(), &self.0)
            .clicked()
        {
            *selected_name = self.0.clone();
        }
    }

    fn children_ui(&mut self, ui: &mut Ui, depth: usize, selected_name: &mut String) {
        self.1.iter_mut().for_each(|tree| {
            tree.ui(ui, depth, selected_name);
        });
    }
}
