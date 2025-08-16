pub mod login_view;
pub mod home_view;
pub mod relays_view;
pub mod profile_view;
pub mod wallet_view;
pub mod image_cache;
pub mod zap;
pub mod article_view;

use eframe::egui::{self, Margin};
// nostr v0.43.0 / nostr-sdk: RelayMetadata は nostr_sdk::nips::nip65 に移動したため import する
use crate::{
    NostrStatusApp,
    theme::{dark_visuals, light_visuals},
    types::*,
};

use crate::nostr_client;

impl eframe::App for NostrStatusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut app_data = self.data.lock().unwrap();

        // --- Article Fetching Logic ---
        if let Some(event_id) = app_data.viewing_article_id {
            if app_data.viewing_article.is_none() && !app_data.is_loading {
                app_data.is_loading = true;
                app_data.should_repaint = true;

                let app_data_arc = self.data.clone();
                let runtime = self.runtime.handle().clone();
                runtime.spawn(async move {
                    let (cache_db, client) = {
                        let data = app_data_arc.lock().unwrap();
                        (data.cache_db.clone(), data.nostr_client.as_ref().unwrap().clone())
                    };

                    let result = nostr_client::fetch_article(&cache_db, &client, event_id).await;

                    let mut data = app_data_arc.lock().unwrap();
                    match result {
                        Ok(article) => {
                            data.viewing_article = Some(article);
                        }
                        Err(e) => {
                            eprintln!("Failed to fetch article: {}", e);
                            // Optionally, reset the view or show an error
                            data.viewing_article_id = None;
                            data.current_tab = AppTab::Home;
                        }
                    }
                    data.is_loading = false;
                    data.should_repaint = true;
                });
            }
        }


        let home_tab_text = "ホーム";

        // app_data_arc をクローンして非同期タスクに渡す
        let app_data_arc_clone = self.data.clone();
        let runtime_handle = self.runtime.handle().clone();

        let mut urls_to_load: Vec<(String, ImageKind)> = Vec::new();

        let panel_frame = egui::Frame::default()
            .inner_margin(Margin::same(15))
            .fill(ctx.style().visuals.panel_fill);

        egui::SidePanel::left("side_panel")
            .frame(panel_frame)
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.heading("note");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (icon, new_theme) = match app_data.current_theme {
                            AppTheme::Light => ("☀️", AppTheme::Dark),
                            AppTheme::Dark => ("🌙", AppTheme::Light),
                        };
                        if ui.button(icon).clicked() {
                            app_data.current_theme = new_theme;
                            let new_visuals = match new_theme {
                                AppTheme::Light => light_visuals(),
                                AppTheme::Dark => dark_visuals(),
                            };
                            ctx.set_visuals(new_visuals);
                        }
                    });
                });

                ui.add_space(15.0);

                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    ui.style_mut().spacing.item_spacing.y = 12.0;

                    ui.selectable_value(&mut app_data.current_tab, AppTab::Home, home_tab_text);

                    ui.add_space(15.0);
                    ui.label(egui::RichText::new("ラベル一覧").strong());
                    ui.add_space(10.0);

                    // Placeholder labels
                    let labels = vec!["すべて", "テクノロジー", "音楽", "Rust", "Nostr"];
                    for label in labels {
                        let is_selected = app_data.selected_label == Some(label.to_string());
                        if ui.selectable_label(is_selected, label).clicked() {
                            if is_selected {
                                // If clicked again, deselect
                                app_data.selected_label = None;
                            } else {
                                app_data.selected_label = Some(label.to_string());
                            }
                        }
                    }
                });

                if app_data.is_logged_in {
                    ui.add_space(20.0);

                    // --- 投稿ボタン ---
                    let post_button_text = egui::RichText::new("投稿する").size(14.0).strong();
                    let button = egui::Button::new(post_button_text)
                        .min_size(egui::vec2(ui.available_width(), 40.0))
                        .corner_radius(egui::CornerRadius::from(8.0));

                    if ui.add(button).clicked() {
                        app_data.show_post_dialog = true;
                    }
                }
            });

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                // NEW Top Panel for profile icon menu
                egui::TopBottomPanel::top("top_panel")
                    .frame(egui::Frame::default().inner_margin(Margin::symmetric(10, 5)))
                    .show_inside(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if app_data.is_logged_in {
                                let avatar_size = egui::vec2(40.0, 40.0); // Increased size
                                let avatar_url = app_data.editable_profile.picture.clone();
                                let image_state = if !avatar_url.is_empty() {
                                    app_data.image_cache.get(&avatar_url).cloned()
                                } else {
                                    None
                                };

                                let response;
                                match image_state {
                                    Some(ImageState::Loaded(texture_handle)) => {
                                        let img_button = egui::ImageButton::new(egui::Image::new(&texture_handle).max_size(avatar_size));
                                        response = ui.add(img_button);
                                    },
                                    _ => {
                                        if !avatar_url.is_empty() && !urls_to_load.iter().any(|(u, _)| u == &avatar_url) {
                                            urls_to_load.push((avatar_url.clone(), ImageKind::Avatar));
                                        }
                                        let button = egui::Button::new("👤").min_size(avatar_size);
                                        response = ui.add(button);
                                    }
                                };

                                if response.clicked() {
                                    app_data.show_profile_menu = !app_data.show_profile_menu;
                                }

                                if app_data.show_profile_menu {
                                    egui::Area::new("profile_menu_area".into())
                                        .fixed_pos(response.rect.left_bottom())
                                        .show(ctx, |ui| {
                                            let frame = egui::Frame {
                                                inner_margin: egui::Margin::same(10),
                                                ..egui::Frame::menu(ui.style())
                                            };
                                            frame.show(ui, |ui| {
                                                if ui.button("Profile").clicked() {
                                                    app_data.current_tab = AppTab::Profile;
                                                    app_data.current_profile_sub_view = ProfileSubView::Profile;
                                                    app_data.show_profile_menu = false;
                                                }
                                                if ui.button("Relays").clicked() {
                                                    app_data.current_tab = AppTab::Profile;
                                                    app_data.current_profile_sub_view = ProfileSubView::Relays;
                                                    app_data.show_profile_menu = false;
                                                }
                                                if ui.button("Wallet").clicked() {
                                                    app_data.current_tab = AppTab::Profile;
                                                    app_data.current_profile_sub_view = ProfileSubView::Wallet;
                                                    app_data.show_profile_menu = false;
                                                }
                                            });
                                        });

                                    // Close menu if clicking outside
                                    if ctx.input(|i| i.pointer.any_click() && !response.rect.contains(i.pointer.interact_pos().unwrap_or_default())) {
                                        app_data.show_profile_menu = false;
                                    }
                                }
                            }
                        });
                    });

                if !app_data.is_logged_in {
                    if app_data.current_tab == AppTab::Home {
                        login_view::draw_login_view(ui, &mut app_data, app_data_arc_clone.clone(), runtime_handle.clone());
                    }
                } else {
                    match app_data.current_tab {
                        AppTab::Home => {
                            home_view::draw_home_view(ui, ctx, &mut app_data, app_data_arc_clone.clone(), runtime_handle.clone(), &mut urls_to_load);
                        },
                        AppTab::Profile => {
                            // NEW: Sub-view matching
                            match app_data.current_profile_sub_view {
                                ProfileSubView::Profile => {
                                    profile_view::draw_profile_view(ui, ctx, &mut app_data, app_data_arc_clone.clone(), runtime_handle.clone(), &mut urls_to_load);
                                },
                                ProfileSubView::Relays => {
                                    relays_view::draw_relays_view(ui, &mut app_data, app_data_arc_clone.clone(), runtime_handle.clone());
                                },
                                ProfileSubView::Wallet => {
                                    wallet_view::draw_wallet_view(ui, &mut app_data, app_data_arc_clone.clone(), runtime_handle.clone());
                                },
                            }
                        },
                        AppTab::ArticleView => {
                            article_view::draw_article_view(ui, ctx, &mut app_data, app_data_arc_clone.clone(), runtime_handle.clone(), &mut urls_to_load);
                        }
                    }
                }
        });

        // --- Image Loading Logic ---
        let cache_db = app_data.cache_db.clone();
        let mut still_to_load = Vec::new();
        for (url_key, kind) in urls_to_load {
            if let Some(image_bytes) = image_cache::load_from_lmdb(&cache_db, &url_key) {
                if let Ok(mut dynamic_image) = image::load_from_memory(&image_bytes) {
                    let (width, height) = match kind {
                        ImageKind::Avatar => (32, 32),
                        ImageKind::ProfilePicture => (100, 100),
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
                    app_data.image_cache.insert(url_key, ImageState::Failed);
                }
            } else {
                still_to_load.push((url_key, kind));
            }
        }

        let data_clone = self.data.clone();
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
                            image_cache::save_to_lmdb(&cache_db_for_fetch, &response.url, &response.bytes);
                            match image::load_from_memory(&response.bytes) {
                                Ok(mut dynamic_image) => {
                                    let (width, height) = match kind {
                                        ImageKind::Avatar => (32, 32),
                                        ImageKind::ProfilePicture => (100, 100),
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

        // update メソッドの最後に should_repaint をチェックし、再描画をリクエスト
        if app_data.should_repaint {
            ctx.request_repaint();
            app_data.should_repaint = false; // リクエスト後にフラグをリセット
        }

        // ロード中もUIを常に更新するようリクエスト
        if app_data.is_loading {
            ctx.request_repaint();
        }
    }
}
