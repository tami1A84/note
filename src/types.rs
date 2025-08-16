use eframe::egui;
use egui_commonmark::CommonMarkCache;
use nostr::{nips::nip47::NostrWalletConnectURI, PublicKey, Timestamp, Keys, EventId, Kind};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use nostr_sdk::Client;

use crate::cache_db::LmdbCache;

// --- Pub-used structs and enums ---

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub encrypted_secret_key: String,
    pub salt: String,
    #[serde(default)]
    pub encrypted_nwc_uri: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Cache<T> {
    pub timestamp: DateTime<Utc>,
    pub data: T,
}

impl<T> Cache<T> {
    pub fn new(data: T) -> Self {
        Self {
            timestamp: Utc::now(),
            data,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        // 24 hours
        let duration = now.signed_duration_since(self.timestamp);
        duration.num_seconds() > (24 * 60 * 60)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProfileMetadata {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub about: String,
    #[serde(default)]
    pub picture: String,
    #[serde(default)]
    pub nip05: String,
    #[serde(default)]
    pub lud16: String,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default)]
pub struct EditableRelay {
    pub url: String,
    pub read: bool,
    pub write: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum ImageKind {
    Avatar,
    ProfilePicture,
}

#[derive(Clone)]
pub enum ImageState {
    Loading,
    Loaded(egui::TextureHandle),
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapReceipt {
    pub id: EventId,
    pub zapper_pubkey: Option<PublicKey>,
    pub recipient_pubkey: PublicKey,
    pub recipient_metadata: ProfileMetadata,
    pub amount_msats: u64,
    pub created_at: Timestamp,
    pub note: String,
    pub zapped_event_id: Option<EventId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePost {
    pub id: EventId,
    pub kind: Kind,
    pub author_pubkey: PublicKey,
    pub author_metadata: ProfileMetadata,
    pub title: String,
    pub summary: String,
    pub created_at: Timestamp,
    #[serde(default)]
    pub tags: Vec<nostr::Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleFull {
    pub id: EventId,
    pub kind: Kind,
    pub author_pubkey: PublicKey,
    pub author_metadata: ProfileMetadata,
    pub title: String,
    pub content: String,
    pub created_at: Timestamp,
    #[serde(default)]
    pub tags: Vec<nostr::Tag>,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ProfileSubView {
    Profile,
    Relays,
    Wallet,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum AppTab {
    Home,
    Profile,
    ArticleView,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum AppTheme {
    Light,
    Dark,
}

impl AppTheme {
    pub fn card_background_color(&self) -> egui::Color32 {
        match self {
            AppTheme::Light => egui::Color32::from_white_alpha(250),
            AppTheme::Dark => egui::Color32::from_rgb(44, 44, 46),
        }
    }

    pub fn danger_zone_background_color(&self) -> egui::Color32 {
        match self {
            AppTheme::Light => egui::Color32::from_rgb(255, 235, 238),
            AppTheme::Dark => egui::Color32::from_rgb(60, 40, 40),
        }
    }

    pub fn danger_zone_stroke_color(&self) -> egui::Color32 {
        match self {
            AppTheme::Light => egui::Color32::from_rgb(255, 180, 180),
            AppTheme::Dark => egui::Color32::from_rgb(120, 60, 60),
        }
    }
}

pub struct NostrStatusAppInternal {
    pub nwc_uri_input: String,
    pub cache_db: LmdbCache,
    pub is_logged_in: bool,
    pub article_title_input: String,
    pub article_content_input: String,
    pub show_post_dialog: bool,
    pub secret_key_input: String,
    pub passphrase_input: String,
    pub confirm_passphrase_input: String,
    pub nostr_client: Option<Client>,
    pub my_keys: Option<Keys>,
    pub followed_pubkeys: HashSet<PublicKey>,
    pub followed_pubkeys_display: String,
    pub timeline_posts: Vec<TimelinePost>,
    pub should_repaint: bool,
    pub is_loading: bool,
    pub current_tab: AppTab,
    pub current_profile_sub_view: ProfileSubView,
    pub selected_label: Option<String>,
    pub connected_relays_display: String,
    pub nip01_profile_display: String,
    pub editable_profile: ProfileMetadata,
    pub profile_fetch_status: String,
    pub nip65_relays: Vec<EditableRelay>,
    pub discover_relays_editor: String,
    pub default_relays_editor: String,
    pub current_theme: AppTheme,
    pub image_cache: HashMap<String, ImageState>,

    // NWC
    pub nwc_passphrase_input: String,
    pub nwc: Option<NostrWalletConnectURI>,
    pub nwc_client: Option<Client>,
    pub nwc_error: Option<String>,
    pub zap_history: Vec<ZapReceipt>,
    pub zap_history_fetch_status: String,
    pub is_fetching_zap_history: bool,
    // ZAP
    pub show_zap_dialog: bool,
    pub zap_amount_input: String,
    pub zap_target_post: Option<TimelinePost>,
    // Article View
    pub viewing_article_id: Option<EventId>,
    pub viewing_article: Option<ArticleFull>,
    // UI State
    pub show_profile_menu: bool,
    pub commonmark_cache: CommonMarkCache,
}
