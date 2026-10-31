//! Sticker and sticker pack resource endpoints.
//!
//! Official Discord API reference:
//! <https://docs.discord.com/developers/resources/sticker>
//!
//! Implements all 3 sticker and sticker-pack operations:
//! - Get Sticker (`GET /stickers/{sticker_id}`)
//! - List Nitro Sticker Packs (`GET /sticker-packs`)
//! - Get Sticker Pack (`GET /sticker-packs/{pack_id}`)

use crate::client::RestClient;
use crate::error::Error;
use crate::message::{de_opt_snowflake, de_snowflake};
use crate::transport::Transport;
use rift_core::Snowflake;
use serde::Deserialize;

// ───────────────────────── Sticker Models ─────────────────────────

/// Type of sticker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum StickerType {
    /// An official sticker in a pack, part of Nitro or in a removed purchasable pack.
    Standard = 1,
    /// A sticker uploaded to a guild for the guild's members.
    Guild = 2,
}

impl StickerType {
    /// Parse from integer.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Standard),
            2 => Some(Self::Guild),
            _ => None,
        }
    }
    /// As integer.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Sticker format type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum StickerFormatType {
    /// PNG format.
    Png = 1,
    /// Animated PNG format.
    Apng = 2,
    /// Lottie JSON vector format.
    Lottie = 3,
    /// GIF format.
    Gif = 4,
}

impl StickerFormatType {
    /// Parse from integer.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Png),
            2 => Some(Self::Apng),
            3 => Some(Self::Lottie),
            4 => Some(Self::Gif),
            _ => None,
        }
    }
    /// As integer.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Sticker object representing a standard sticker or a guild sticker.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Sticker {
    /// Snowflake ID of the sticker.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// For standard stickers, ID of the pack the sticker is from.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "crate::message::ser_opt_snowflake")]
    pub pack_id: Option<Snowflake>,
    /// Name of the sticker.
    pub name: String,
    /// Description of the sticker.
    #[serde(default)]
    pub description: Option<String>,
    /// Autocomplete/suggestion tags for the sticker (comma-separated string).
    pub tags: String,
    /// Type of sticker.
    #[serde(rename = "type", default)]
    pub kind: Option<u8>,
    /// Type of sticker format.
    #[serde(default)]
    pub format_type: Option<u8>,
    /// Whether this guild sticker can be used, may be false due to loss of Server Boosts.
    #[serde(default)]
    pub available: Option<bool>,
    /// ID of the guild that owns this sticker.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "crate::message::ser_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// The user that uploaded the guild sticker.
    #[serde(default)]
    pub user: Option<crate::interaction::User>,
    /// The standard sticker's sort order within its pack.
    #[serde(default)]
    pub sort_value: Option<i32>,
}

/// Collection of Nitro sticker packs (`GET /sticker-packs`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct StickerPackCollection {
    /// List of sticker packs.
    pub sticker_packs: Vec<StickerPack>,
}

/// A sticker pack containing standard Nitro stickers.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct StickerPack {
    /// Snowflake ID of the sticker pack.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// The stickers in the pack.
    pub stickers: Vec<Sticker>,
    /// Name of the sticker pack.
    pub name: String,
    /// ID of the pack's SKU.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub sku_id: Snowflake,
    /// ID of a sticker in the pack which is shown as the pack's icon.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "crate::message::ser_opt_snowflake")]
    pub cover_sticker_id: Option<Snowflake>,
    /// Description of the sticker pack.
    #[serde(default)]
    pub description: Option<String>,
    /// ID of the sticker pack's banner image.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "crate::message::ser_opt_snowflake")]
    pub banner_asset_id: Option<Snowflake>,
}

// ───────────────────────── REST Methods ─────────────────────────

impl<T: Transport> RestClient<T> {
    /// `GET /stickers/{sticker_id}` – Returns a sticker object for the given sticker ID.
    pub async fn get_sticker(&self, sticker_id: Snowflake) -> Result<Sticker, Error> {
        let path = format!("/stickers/{sticker_id}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /sticker-packs` – Returns the list of sticker packs available to Nitro subscribers.
    pub async fn list_sticker_packs(&self) -> Result<StickerPackCollection, Error> {
        self.request("GET", "/sticker-packs", None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /sticker-packs/{pack_id}` – Returns a sticker pack for the given pack ID.
    pub async fn get_sticker_pack(&self, pack_id: Snowflake) -> Result<StickerPack, Error> {
        let path = format!("/sticker-packs/{pack_id}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }
}

// ───────────────────────── Tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::RestClient;
    use crate::transport::{RawResponse, Request, Transport};
    use rift_core::Snowflake;
    use rift_http::Config;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::future::Future;
    use std::sync::Mutex;

    struct Fake {
        queue: Mutex<VecDeque<Result<RawResponse, String>>>,
        seen: Mutex<Vec<Request>>,
    }

    impl Fake {
        fn new(items: Vec<Result<RawResponse, String>>) -> Self {
            Self {
                queue: Mutex::new(items.into()),
                seen: Mutex::new(Vec::new()),
            }
        }
    }

    impl Transport for Fake {
        fn send(&self, req: Request) -> impl Future<Output = Result<RawResponse, String>> + Send {
            self.seen.lock().unwrap().push(req);
            let r = self
                .queue
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Err("queue empty".into()));
            async move { r }
        }
    }

    fn client(responses: Vec<Result<RawResponse, String>>) -> RestClient<Fake> {
        RestClient::with_transport(
            Fake::new(responses),
            "tok",
            "DiscordBot (https://example.com, 1.0.0)".into(),
            Config::default(),
        )
    }

    fn resp(status: u16, body: &str) -> Result<RawResponse, String> {
        Ok(RawResponse {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        })
    }

    fn sf(n: u64) -> Snowflake {
        Snowflake::new(n)
    }

    #[test]
    fn sticker_types_parsing() {
        assert_eq!(StickerType::from_u8(1), Some(StickerType::Standard));
        assert_eq!(StickerType::from_u8(2), Some(StickerType::Guild));
        assert_eq!(StickerType::from_u8(99), None);
        assert_eq!(StickerFormatType::from_u8(1), Some(StickerFormatType::Png));
        assert_eq!(StickerFormatType::from_u8(2), Some(StickerFormatType::Apng));
        assert_eq!(StickerFormatType::from_u8(3), Some(StickerFormatType::Lottie));
        assert_eq!(StickerFormatType::from_u8(4), Some(StickerFormatType::Gif));
    }

    #[tokio::test(start_paused = true)]
    async fn sticker_and_packs_routes() {
        let sticker_json = json!({
            "id": "1111",
            "name": "Wumpus Wave",
            "tags": "wumpus,hello",
            "type": 1,
            "format_type": 1
        })
        .to_string();

        let pack_json = json!({
            "id": "2222",
            "sku_id": "3333",
            "name": "Classic Wumpus",
            "stickers": [
                {
                    "id": "1111",
                    "name": "Wumpus Wave",
                    "tags": "wumpus,hello",
                    "type": 1,
                    "format_type": 1
                }
            ]
        })
        .to_string();

        let collection_json = json!({
            "sticker_packs": [
                {
                    "id": "2222",
                    "sku_id": "3333",
                    "name": "Classic Wumpus",
                    "stickers": []
                }
            ]
        })
        .to_string();

        let c = client(vec![
            resp(200, &sticker_json),
            resp(200, &collection_json),
            resp(200, &pack_json),
        ]);

        let s = c.get_sticker(sf(1111)).await.unwrap();
        assert_eq!(s.id, sf(1111));
        assert_eq!(s.name, "Wumpus Wave");

        let packs = c.list_sticker_packs().await.unwrap();
        assert_eq!(packs.sticker_packs.len(), 1);
        assert_eq!(packs.sticker_packs[0].id, sf(2222));

        let pack = c.get_sticker_pack(sf(2222)).await.unwrap();
        assert_eq!(pack.id, sf(2222));
        assert_eq!(pack.stickers.len(), 1);

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].url, "https://discord.com/api/v10/stickers/1111");
        assert_eq!(seen[0].method, "GET");

        assert_eq!(seen[1].url, "https://discord.com/api/v10/sticker-packs");
        assert_eq!(seen[1].method, "GET");

        assert_eq!(seen[2].url, "https://discord.com/api/v10/sticker-packs/2222");
        assert_eq!(seen[2].method, "GET");
    }
}
