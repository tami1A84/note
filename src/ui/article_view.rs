use eframe::egui;
use std::sync::{Arc, Mutex};
use nostr::prelude::ToBech32;
use egui_commonmark::CommonMarkViewer;

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
            CommonMarkViewer::new().show(ui, &mut app_data.commonmark_cache, &post.content);
        });

    } else {
        // Article is being loaded, show a spinner
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 2.0 - 20.0); // Center vertically
            ui.spinner();
            ui.add_space(10.0);
            ui.label("記事を読み込んでいます...");
        });

        // Also provide a way to go back if it gets stuck
        if ui.button("← Back").clicked() {
            app_data.viewing_article_id = None;
            app_data.viewing_article = None;
            app_data.current_tab = AppTab::Home;
        }
    }
}
