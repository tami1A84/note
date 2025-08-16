use eframe::egui;
use std::sync::{Arc, Mutex};
use nostr::prelude::ToBech32;

use crate::types::*;

pub fn draw_article_view(
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
    app_data: &mut NostrStatusAppInternal,
    _app_data_arc: Arc<Mutex<NostrStatusAppInternal>>,
    _runtime_handle: tokio::runtime::Handle,
    _urls_to_load: &mut Vec<(String, ImageKind)>,
) {
    if let Some(post) = &app_data.viewing_article {
        // Back button
        if ui.button("← Back").clicked() {
            app_data.viewing_article = None;
            app_data.current_tab = AppTab::Home;
            return;
        }
        ui.separator();
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Constrain the width for readability
            ui.set_max_width(700.0);

            // Big Title
            ui.heading(&post.title);
            ui.add_space(5.0);

            // Author info
            ui.horizontal(|ui| {
                // Simplified author display for now
                let display_name = if !post.author_metadata.name.is_empty() {
                    post.author_metadata.name.clone()
                } else {
                    let pubkey = post.author_pubkey.to_bech32().unwrap_or_default();
                    format!("{}...{}", &pubkey[0..8], &pubkey[pubkey.len()-4..])
                };
                ui.label("by");
                ui.label(egui::RichText::new(display_name).strong());
            });
            ui.add_space(15.0);
            ui.separator();
            ui.add_space(15.0);

            // Full Content
            // For now, just a simple label. Could be extended with Markdown rendering.
            ui.label(&post.content);
        });

    } else {
        // This case should ideally not be reached if logic is correct
        ui.label("No article selected.");
        if ui.button("Go Home").clicked() {
            app_data.current_tab = AppTab::Home;
        }
    }
}
