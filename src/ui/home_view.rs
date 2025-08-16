use eframe::egui;
use std::sync::{Arc, Mutex};
use nostr::{EventBuilder, Kind, PublicKey, Tag, nips::nip19::ToBech32, EventId};
use regex::Regex;

use crate::{
    types::*,
    nostr_client::{update_contact_list, fetch_timeline_events},
    cache_db::DB_FOLLOWED,
    ui::{image_cache, zap},
};

pub fn draw_home_view(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app_data: &mut NostrStatusAppInternal,
    app_data_arc: Arc<Mutex<NostrStatusAppInternal>>,
    runtime_handle: tokio::runtime::Handle,
) {
    let mut urls_to_load: Vec<(String, ImageKind)> = Vec::new();
    let new_post_window_title_text = "新規投稿";
    let publish_button_text = "公開";
    let cancel_button_text = "キャンセル";
    let timeline_heading_text = "ホーム";
    let fetch_latest_button_text = "最新の投稿を取得";
    let no_timeline_message_text = "タイムラインに投稿はまだありません。";

    let card_frame = egui::Frame {
        inner_margin: egui::Margin::same(12),
        corner_radius: 8.0.into(),
        shadow: eframe::epaint::Shadow::NONE,
        fill: app_data.current_theme.card_background_color(),
        ..Default::default()
    };

    // --- ZAP Dialog ---
    if app_data.show_zap_dialog {
        if let Some(post_to_zap) = app_data.zap_target_post.clone() {
            let mut close_dialog = false;
            egui::Window::new("ZAPを送る")
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.add_space(10.0);
                        let display_name = if !post_to_zap.author_metadata.name.is_empty() {
                            post_to_zap.author_metadata.name.clone()
                        } else {
                            let pubkey = post_to_zap.author_pubkey.to_bech32().unwrap_or_default();
                            format!("{}...{}", &pubkey[0..8], &pubkey[pubkey.len()-4..])
                        };
                        ui.label(format!("{} にZAPします", display_name));
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label("金額 (sats):");
                            ui.add(egui::TextEdit::singleline(&mut app_data.zap_amount_input)
                                .desired_width(120.0));
                        });
                        ui.add_space(10.0);
                    });

                    ui.separator();
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        if ui.button("キャンセル").clicked() {
                           close_dialog = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("ZAP").clicked() {
                                if let (Some(nwc), Some(nwc_client), Some(my_keys)) =
                                    (app_data.nwc.as_ref(), app_data.nwc_client.as_ref(), app_data.my_keys.as_ref())
                                {
                                    if let Ok(amount_sats) = app_data.zap_amount_input.parse::<u64>() {
                                        let nwc_clone = nwc.clone();
                                        let nwc_client_clone = nwc_client.clone();
                                        let my_keys_clone = my_keys.clone();
                                        let app_data_clone = app_data_arc.clone();

                                        runtime_handle.spawn(async move {
                                            {
                                                let mut data = app_data_clone.lock().unwrap();
                                                data.should_repaint = true;
                                            } // Lock is dropped here

                                            let result = zap::send_zap_request(
                                                &nwc_clone,
                                                &nwc_client_clone,
                                                &my_keys_clone,
                                                post_to_zap.author_pubkey,
                                                &post_to_zap.author_metadata.lud16,
                                                amount_sats,
                                                Some(post_to_zap.id),
                                                Some(post_to_zap.kind),
                                            ).await;

                                            let mut data = app_data_clone.lock().unwrap();
                                            match result {
                                                Ok(_) => {
                                                    // ZAPリクエストを送信しました。ウォレットの確認を待っています...
                                                }
                                                Err(e) => {
                                                    eprintln!("ZAPエラー: {}", e);
                                                }
                                            }
                                            data.should_repaint = true;
                                        });

                                        close_dialog = true;

                                    } else {
                                        eprintln!("無効な金額です");
                                    }
                                } else {
                                    eprintln!("ZAPにはNWCの接続が必要です");
                                }
                            }
                        });
                    });
                });
            if close_dialog {
                app_data.show_zap_dialog = false;
                app_data.zap_target_post = None;
            }
        }
    }


    if app_data.show_post_dialog {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, "dim_layer".into()));
        let screen_rect = ctx.screen_rect();
        painter.add(egui::Shape::rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128)));

        egui::Window::new(new_post_window_title_text)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                egui::TopBottomPanel::bottom("post_dialog_buttons")
                    .show_inside(ui, |ui| {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui.button("😀").clicked() {
                                app_data.show_emoji_picker = !app_data.show_emoji_picker;
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(cancel_button_text).clicked() {
                                    app_data.show_post_dialog = false;
                                app_data.article_title_input.clear();
                                app_data.article_content_input.clear();
                                }
                                if ui.button(publish_button_text).clicked() && !app_data.is_loading {
                                let article_title = app_data.article_title_input.clone();
                                let article_content = app_data.article_content_input.clone();

                                if article_title.is_empty() {
                                    eprintln!("Title cannot be empty.");
                                        return;
                                    }
                                if article_content.is_empty() {
                                    eprintln!("Content cannot be empty.");
                                    return;
                                }

                                let client_clone = app_data.nostr_client.as_ref().unwrap().clone();
                                let keys_clone = app_data.my_keys.clone().unwrap();

                                app_data.is_loading = true;
                                app_data.should_repaint = true;
                                println!("Publishing NIP-23 article...");

                                    let my_emojis = app_data.my_emojis.clone();
                                    let cloned_app_data_arc = app_data_arc.clone();
                                    runtime_handle.spawn(async move {
                                        let mut tags: Vec<Tag> = Vec::new();

                                    // Add the 't' tag for the title, as per NIP-23
                                    tags.push(Tag::from_standardized(nostr::TagStandard::Title(article_title)));

                                    // Emoji tag processing
                                        let re = Regex::new(r":(\w+):").unwrap();
                                        let mut used_emojis: std::collections::HashSet<String> = std::collections::HashSet::new();
                                    for cap in re.captures_iter(&article_content) {
                                            if let Some(shortcode) = cap.get(1) {
                                                used_emojis.insert(shortcode.as_str().to_string());
                                            }
                                        }
                                        for shortcode in used_emojis {
                                            if let Some(url) = my_emojis.get(&shortcode) {
                                                if let Ok(tag) = Tag::parse(["emoji", &shortcode, url]) {
                                                    tags.push(tag);
                                                }
                                            }
                                        }

                                    // Create the NIP-23 event (kind 30023)
                                    let event_result = EventBuilder::new(Kind::from(30023), article_content)
                                            .tags(tags)
                                        .sign(&keys_clone)
                                            .await;

                                        match event_result {
                                        Ok(event) => match client_clone.send_event(&event).await {
                                                Ok(event_id) => {
                                                println!("Article published with event id: {event_id:?}");
                                                    let mut data = cloned_app_data_arc.lock().unwrap();
                                                data.show_post_dialog = false;
                                                data.article_title_input.clear();
                                                data.article_content_input.clear();
                                                }
                                                Err(e) => {
                                                eprintln!("Failed to publish article: {e}");
                                                }
                                            },
                                            Err(e) => {
                                                eprintln!("Failed to create event: {e}");
                                            }
                                        }
                                        let mut data = cloned_app_data_arc.lock().unwrap();
                                        data.is_loading = false;
                                        data.should_repaint = true;
                                    });
                                }
                            });
                        });
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.add_space(10.0);
                    ui.label("タイトル:");
                    ui.add(
                        egui::TextEdit::singleline(&mut app_data.article_title_input)
                            .desired_width(f32::INFINITY)
                            .hint_text("記事のタイトル"),
                    );
                    ui.add_space(5.0);
                    ui.label("本文:");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut app_data.article_content_input)
                                .desired_rows(15)
                                .desired_width(f32::INFINITY)
                                .hint_text("記事の内容をMarkdownで記述..."),
                        );
                    });
                });
            });

        if app_data.show_emoji_picker {
            egui::Window::new("カスタム絵文字")
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 180.0)) // Adjust position to be below the post dialog
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("絵文字を選択");
                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true), |ui| {
                            if app_data.my_emojis.is_empty() {
                                ui.label("カスタム絵文字が設定されていません。");
                            } else {
                                for (shortcode, url) in app_data.my_emojis.clone().into_iter() {
                                    let emoji_size = egui::vec2(24.0, 24.0);
                                    let url_key = url.to_string();

                                    let sense = egui::Sense::click();
                                    let (rect, response) = ui.allocate_exact_size(emoji_size, sense);

                                    if response.hovered() {
                                        ui.painter().rect_filled(rect.expand(2.0), egui::CornerRadius::from(4.0), ui.visuals().widgets.hovered.bg_fill);
                                    }

                                    match app_data.image_cache.get(&url_key) {
                                        Some(ImageState::Loaded(texture_handle)) => {
                                            let image = egui::Image::new(texture_handle).fit_to_exact_size(emoji_size);
                                            image.paint_at(ui, rect);
                                        }
                                        Some(ImageState::Loading) => {
                                            ui.put(rect, egui::Spinner::new());
                                        }
                                        Some(ImageState::Failed) => {
                                            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "💔", egui::FontId::default(), ui.visuals().error_fg_color);
                                        }
                                        None => {
                                            if !urls_to_load.iter().any(|(u, _)| u == &url_key) {
                                                urls_to_load.push((url_key.clone(), ImageKind::Emoji));
                                            }
                                            ui.put(rect, egui::Spinner::new());
                                        }
                                    }

                                    if response.clicked() {
                                        app_data.article_content_input.push_str(&format!(":{}:", shortcode));
                                        app_data.show_emoji_picker = false;
                                    }
                                    response.on_hover_text(&format!(":{}:", shortcode));
                                }
                            }
                        });
                    });
                    if ui.button("閉じる").clicked() {
                        app_data.show_emoji_picker = false;
                    }
                });
        }
    }


    card_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading(timeline_heading_text);

            let fetch_button = egui::Button::new(egui::RichText::new(fetch_latest_button_text).strong());
            if ui.add_enabled(!app_data.is_loading, fetch_button).clicked() {
                let followed_pubkeys = app_data.followed_pubkeys.clone();
                let discover_relays = app_data.discover_relays_editor.clone();
                let my_keys = app_data.my_keys.clone().unwrap();

                app_data.is_loading = true;
                app_data.should_repaint = true;

                let cloned_app_data_arc = app_data_arc.clone();
                runtime_handle.spawn(async move {
                    let timeline_result = fetch_timeline_events(&my_keys, &discover_relays, &followed_pubkeys).await;

                    let mut app_data_async = cloned_app_data_arc.lock().unwrap();
                    app_data_async.is_loading = false;
                    match timeline_result {
                        Ok(new_posts) => {
                            if !new_posts.is_empty() {
                                let mut existing_ids: std::collections::HashSet<EventId> = app_data_async.timeline_posts.iter().map(|p| p.id).collect();
                                let mut added_posts = 0;
                                for post in new_posts {
                                    if !existing_ids.contains(&post.id) {
                                        existing_ids.insert(post.id);
                                        app_data_async.timeline_posts.push(post);
                                        added_posts += 1;
                                    }
                                }

                                if added_posts > 0 {
                                    app_data_async.timeline_posts.sort_by_key(|p| std::cmp::Reverse(p.created_at));
                                    println!("Added {} new statuses to the timeline.", added_posts);
                                } else {
                                    println!("No new statuses found.");
                                }
                            } else {
                                println!("Fetched 0 statuses.");
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to fetch timeline: {e}");
                        }
                    }
                    app_data_async.should_repaint = true;
                });
            }

            if app_data.is_loading {
                ui.add_space(10.0);
                ui.spinner();
                ui.label("更新中...");
            }
        });
        ui.add_space(10.0);
        let pubkey_to_modify: Option<(PublicKey, bool)> = None;

        let heading_text = app_data.selected_label.as_deref().unwrap_or("ホーム");
        ui.heading(heading_text);
        ui.add_space(10.0);

        if app_data.timeline_posts.is_empty() {
            ui.label(no_timeline_message_text);
        } else {
            egui::ScrollArea::horizontal()
                .id_salt("timeline_scroll_area_horizontal")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for post in &app_data.timeline_posts {
                            // Placeholder filter logic
                            let display_post = match &app_data.selected_label {
                                Some(label) if label != "すべて" => post.title.contains(label) || post.content.contains(label),
                                _ => true, // Show all for "すべて" or None
                            };

                            if display_post {
                                let card_frame = egui::Frame {
                                    inner_margin: egui::Margin::same(12),
                                    outer_margin: egui::Margin::symmetric(5, 0),
                                    corner_radius: 8.0.into(),
                                    shadow: eframe::epaint::Shadow::NONE,
                                    fill: app_data.current_theme.card_background_color(),
                                    ..Default::default()
                                };

                                card_frame.show(ui, |ui| {
                                    ui.set_max_width(250.0);
                                    ui.set_max_height(180.0);

                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            let avatar_size = egui::vec2(24.0, 24.0);
                                            let url = &post.author_metadata.picture;
                                            if !url.is_empty() {
                                                let url_key = url.to_string();
                                                let image_state = app_data.image_cache.get(&url_key).cloned();
                                                match image_state {
                                                    Some(ImageState::Loaded(texture_handle)) => {
                                                        let image_widget = egui::Image::new(&texture_handle)
                                                            .corner_radius(4.0)
                                                            .fit_to_exact_size(avatar_size);
                                                        ui.add(image_widget);
                                                    }
                                                    Some(ImageState::Loading) => {
                                                        let (rect, _) = ui.allocate_exact_size(avatar_size, egui::Sense::hover());
                                                        ui.painter().rect_filled(rect, 4.0, ui.style().visuals.widgets.inactive.bg_fill);
                                                        ui.put(rect, egui::Spinner::new());
                                                    }
                                                    Some(ImageState::Failed) => {
                                                        let (rect, _) = ui.allocate_exact_size(avatar_size, egui::Sense::hover());
                                                        ui.painter().rect_filled(rect, 4.0, ui.style().visuals.error_fg_color.linear_multiply(0.2));
                                                    }
                                                    None => {
                                                        if !urls_to_load.iter().any(|(u, _)| u == &url_key) {
                                                            urls_to_load.push((url_key.clone(), ImageKind::Avatar));
                                                        }
                                                        let (rect, _) = ui.allocate_exact_size(avatar_size, egui::Sense::hover());
                                                        ui.painter().rect_filled(rect, 4.0, ui.style().visuals.widgets.inactive.bg_fill);
                                                        ui.put(rect, egui::Spinner::new());
                                                    }
                                                }
                                            } else {
                                                let (rect, _) = ui.allocate_exact_size(avatar_size, egui::Sense::hover());
                                                ui.painter().rect_filled(rect, 4.0, ui.style().visuals.widgets.inactive.bg_fill);
                                            }

                                            let display_name = if !post.author_metadata.name.is_empty() {
                                                post.author_metadata.name.clone()
                                            } else {
                                                let pubkey = post.author_pubkey.to_bech32().unwrap_or_default();
                                                format!("{}...{}", &pubkey[0..8], &pubkey[pubkey.len()-4..])
                                            };
                                            ui.label(egui::RichText::new(display_name).small());
                                        });

                                        ui.add_space(8.0);
                                        ui.separator();
                                        ui.add_space(8.0);

                                        if !post.title.is_empty() {
                                            ui.label(egui::RichText::new(&post.title).strong());
                                        }

                                        let snippet = if post.content.chars().count() > 80 {
                                            let mut truncated: String = post.content.chars().take(80).collect();
                                            truncated.push_str("...");
                                            truncated
                                        } else {
                                            post.content.clone()
                                        };
                                        ui.label(snippet);
                                    });
                                });
                            }
                        }
                    });
                });
        }

        // --- Image Loading Logic ---

        // First, try to load images from the LMDB cache for URLs not in memory.
        let cache_db = app_data.cache_db.clone();
        let mut still_to_load = Vec::new();
        for (url_key, kind) in urls_to_load {
            if let Some(image_bytes) = image_cache::load_from_lmdb(&cache_db, &url_key) {
                // Image found in cache, process it directly.
                // This is a simplification; for a smoother UI, this should be async.
                if let Ok(mut dynamic_image) = image::load_from_memory(&image_bytes) {
                    let (width, height) = match kind {
                        ImageKind::Avatar => (32, 32),
                        ImageKind::Emoji => (20, 20),
                    _ => (32, 32), // Default for Banner, ProfilePicture, etc.
                    };
                    dynamic_image = dynamic_image.thumbnail(width, height);
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [dynamic_image.width() as usize, dynamic_image.height() as usize],
                        dynamic_image.to_rgba8().as_flat_samples().as_slice(),
                    );
                    let texture_handle = ctx.load_texture(
                        &url_key,
                        color_image,
                        Default::default()
                    );
                    app_data.image_cache.insert(url_key, ImageState::Loaded(texture_handle));
                } else {
                    // Failed to decode, mark as failed.
                    app_data.image_cache.insert(url_key, ImageState::Failed);
                }
            } else {
                // Not on disk, queue for network download.
                still_to_load.push((url_key, kind));
            }
        }

        // Fetch remaining images from the network.
        let data_clone = app_data_arc.clone();
        for (url_key, kind) in still_to_load {
            app_data.image_cache.insert(url_key.clone(), ImageState::Loading);
            app_data.should_repaint = true;

            let app_data_clone = data_clone.clone();
            let ctx_clone = ctx.clone();
            let cache_db_for_fetch = app_data.cache_db.clone();
            let request = ehttp::Request::get(&url_key);

            ehttp::fetch(request, move |result| {
                let new_state = match result {
                    Ok(response) => {
                        if response.ok {
                            // Save to LMDB cache first.
                            image_cache::save_to_lmdb(&cache_db_for_fetch, &response.url, &response.bytes);

                            match image::load_from_memory(&response.bytes) {
                                Ok(mut dynamic_image) => {
                                    let (width, height) = match kind {
                                        ImageKind::Avatar => (32, 32),
                                        ImageKind::Emoji => (20, 20),
                                    _ => (32, 32), // Default for Banner, ProfilePicture, etc.
                                    };
                                    dynamic_image = dynamic_image.thumbnail(width, height);

                                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                        [dynamic_image.width() as usize, dynamic_image.height() as usize],
                                        dynamic_image.to_rgba8().as_flat_samples().as_slice(),
                                    );
                                    let texture_handle = ctx_clone.load_texture(
                                        &response.url,
                                        color_image,
                                        Default::default()
                                    );
                                    ImageState::Loaded(texture_handle)
                                }
                                Err(_) => ImageState::Failed,
                            }
                        } else {
                            ImageState::Failed
                        }
                    }
                    Err(_) => ImageState::Failed,
                };

                let mut app_data = app_data_clone.lock().unwrap();
                app_data.image_cache.insert(url_key, new_state);
                ctx_clone.request_repaint();
            });
        }

        if let Some((pubkey, follow)) = pubkey_to_modify {
            if !app_data.is_loading {
                let client = app_data.nostr_client.as_ref().unwrap().clone();
                let keys = app_data.my_keys.as_ref().unwrap().clone();
                let cache_db_clone = app_data.cache_db.clone();

                app_data.is_loading = true;
                app_data.should_repaint = true;

                let cloned_app_data_arc = app_data_arc.clone();
                runtime_handle.spawn(async move {
                    match update_contact_list(&client, &keys, pubkey, follow).await {
                        Ok(new_followed_pubkeys) => {
                            let mut app_data = cloned_app_data_arc.lock().unwrap();
                            app_data.followed_pubkeys = new_followed_pubkeys;
                            if let Some(keys) = &app_data.my_keys {
                                let pubkey_hex = keys.public_key().to_string();
                                if let Err(e) = cache_db_clone.write_cache(DB_FOLLOWED, &pubkey_hex, &app_data.followed_pubkeys) {
                                    eprintln!("Failed to write follow list cache: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to update contact list: {e}");
                        }
                    }
                    let mut app_data = cloned_app_data_arc.lock().unwrap();
                    app_data.is_loading = false;
                    app_data.should_repaint = true;
                });
            }
        }
    });

}
