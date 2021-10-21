use eframe::egui::{self, popup_below_widget, Key};

pub struct AcState {
    /// Selection index in the autocomplet list
    select: Option<usize>,
    /// Input changed this frame
    pub input_changed: bool,
}

impl Default for AcState {
    fn default() -> Self {
        Self {
            select: Some(0),
            input_changed: true,
        }
    }
}

/// Popup for autocompleting.
///
/// Returns whether a suggestion was applied or not.
pub(super) fn autocomplete_popup(
    string: &mut String,
    state: &mut AcState,
    candidates: &[&str],
    ui: &mut egui::Ui,
    response: &egui::Response,
) -> bool {
    let input = ui.input();
    let popup_id = ui.make_persistent_id("autocomplete_popup");
    let mut last = string.split_ascii_whitespace().last().unwrap_or("");
    // Ignore '!' character
    if last.bytes().next() == Some(b'!') {
        last = &last[1..];
    }
    if input.key_pressed(Key::ArrowDown) {
        match &mut state.select {
            None => state.select = Some(0),
            Some(sel) => *sel += 1,
        }
    }
    if let Some(sel) = &mut state.select {
        if input.key_pressed(Key::ArrowUp) {
            if *sel > 0 {
                *sel -= 1;
            } else {
                // Allow selecting "Nothing" by going above first element
                state.select = None;
            }
        }
    } else if state.input_changed {
        // Always select index 0 when input was changed for convenience
        state.select = Some(0);
    }
    if !string.is_empty() {
        let mut exact_match = None;
        // Get length of list and also whether there is an exact match
        let mut i = 0;
        let len = candidates
            .iter()
            .filter(|candidate| {
                if **candidate == last {
                    exact_match = Some(i);
                }
                let predicate = candidate.contains(last);
                if predicate {
                    i += 1;
                }
                predicate
            })
            .count();
        match exact_match {
            Some(idx) if state.input_changed => state.select = Some(idx),
            _ => {}
        }
        if len > 0 {
            if let Some(selection) = &mut state.select {
                if *selection >= len {
                    *selection = len - 1;
                }
            }
            let mut complete = None;
            popup_below_widget(ui, popup_id, response, |ui| {
                for (i, candidate) in candidates
                    .iter()
                    .filter(|candidate| candidate.contains(last))
                    .enumerate()
                {
                    if ui
                        .selectable_label(state.select == Some(i), candidate)
                        .clicked()
                    {
                        complete = Some(candidate);
                    }
                    if state.select == Some(i)
                        && (input.key_pressed(Key::Tab) || input.key_pressed(Key::Enter))
                    {
                        complete = Some(candidate);
                    }
                }
            });
            if let Some(candidate) = complete {
                let range = str_range(string, last);
                string.replace_range(range, candidate);
                state.input_changed = false;
                return true;
            }
            if !string.is_empty() {
                ui.memory().open_popup(popup_id);
            } else {
                ui.memory().close_popup();
            }
        }
    }
    state.input_changed = false;
    false
}

fn str_range(parent: &str, sub: &str) -> core::ops::Range<usize> {
    let beg = sub.as_ptr() as usize - parent.as_ptr() as usize;
    let end = beg + sub.len();
    beg..end
}
