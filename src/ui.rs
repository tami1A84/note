pub mod login_view;
pub mod home_view;
pub mod relays_view;
pub mod profile_view;
pub mod wallet_view;
pub mod image_cache;
pub mod zap;

use eframe::egui::{self, Margin};
// nostr v0.43.0 / nostr-sdk: RelayMetadata は nostr_sdk::nips::nip65 に移動したため import する
use crate::{
    NostrStatusApp,
    theme::{dark_visuals, light_visuals},
    types::*,
};

impl eframe::App for NostrStatusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut app_data = self.data.lock().unwrap();

        let home_tab_text = "ホーム";

        // app_data_arc をクローンして非同期タスクに渡す
        let app_data_arc_clone = self.data.clone();
        let runtime_handle = self.runtime.handle().clone();

        let panel_frame = egui::Frame::default()
            .inner_margin(Margin::same(15))
            .fill(ctx.style().visuals.panel_fill);

        egui::SidePanel::left("side_panel")
            .frame(panel_frame)
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.heading("長文ノート");
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
                                ui.menu_button("👤", |ui| {
                                    if ui.button("Profile").clicked() {
                                        app_data.current_tab = AppTab::Profile;
                                        app_data.current_profile_sub_view = ProfileSubView::Profile;
                                        ui.close();
                                    }
                                    if ui.button("Relays").clicked() {
                                        app_data.current_tab = AppTab::Profile;
                                        app_data.current_profile_sub_view = ProfileSubView::Relays;
                                        ui.close();
                                    }
                                    if ui.button("Wallet").clicked() {
                                        app_data.current_tab = AppTab::Profile;
                                        app_data.current_profile_sub_view = ProfileSubView::Wallet;
                                        ui.close();
                                    }
                                });
                            }
                        });
                    });

                if !app_data.is_logged_in {
                    if app_data.current_tab == AppTab::Home {
                        login_view::draw_login_view(ui, &mut app_data, app_data_arc_clone, runtime_handle);
                    }
                } else {
                    match app_data.current_tab {
                        AppTab::Home => {
                            home_view::draw_home_view(ui, ctx, &mut app_data, app_data_arc_clone, runtime_handle);
                        },
                        AppTab::Profile => {
                            // NEW: Sub-view matching
                            match app_data.current_profile_sub_view {
                                ProfileSubView::Profile => {
                                    profile_view::draw_profile_view(ui, ctx, &mut app_data, app_data_arc_clone, runtime_handle);
                                },
                                ProfileSubView::Relays => {
                                    relays_view::draw_relays_view(ui, &mut app_data, app_data_arc_clone, runtime_handle);
                                },
                                ProfileSubView::Wallet => {
                                    wallet_view::draw_wallet_view(ui, &mut app_data, app_data_arc_clone, runtime_handle);
                                },
                            }
                        },
                        // All cases are handled, no fallback needed
                    }
                }
        });

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
