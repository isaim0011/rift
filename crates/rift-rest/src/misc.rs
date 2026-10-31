//! Miscellaneous REST resource endpoints.
//!
//! Covers:
//! - Soundboard Default Sounds (`GET /soundboard-default-sounds`)
//! - Voice Regions (`GET /voice/regions`)
//! - SKU Subscriptions (`GET /skus/{sku_id}/subscriptions`)
//! - SKU Subscription (`GET /skus/{sku_id}/subscriptions/{subscription_id}`)
//! - Gateway (`GET /gateway`)

use crate::client::RestClient;
use crate::error::Error;
use crate::message::{de_opt_snowflake, de_snowflake};
use crate::transport::Transport;
use rift_core::Snowflake;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

fn de_vec_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Snowflake>, D::Error> {
    let strings = Vec::<String>::deserialize(d)?;
    strings
        .into_iter()
        .map(|s| s.parse::<Snowflake>().map_err(serde::de::Error::custom))
        .collect()
}

fn de_opt_vec_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<Snowflake>>, D::Error> {
    let opt = Option::<Vec<String>>::deserialize(d)?;
    match opt {
        Some(strings) => {
            let res = strings
                .into_iter()
                .map(|s| s.parse::<Snowflake>().map_err(serde::de::Error::custom))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Some(res))
        }
        None => Ok(None),
    }
}

fn query_string<T: Serialize>(v: &T) -> String {
    let j = serde_json::to_value(v).unwrap_or_default();
    let Some(obj) = j.as_object() else { return String::new() };
    let parts: Vec<String> = obj
        .iter()
        .filter_map(|(k, v)| {
            let s = match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => return None,
            };
            Some(format!("{k}={s}"))
        })
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

// ───────────────────────── Soundboard Models ─────────────────────────

/// A soundboard sound item (`GET /soundboard-default-sounds`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SoundboardSound {
    /// Name of this sound.
    pub name: String,
    /// Snowflake ID of this sound.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub sound_id: Snowflake,
    /// Volume of this sound, from 0 to 1.
    pub volume: f64,
    /// ID of this sound's custom emoji.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "crate::message::ser_opt_snowflake")]
    pub emoji_id: Option<Snowflake>,
    /// Unicode character of this sound's standard emoji.
    #[serde(default)]
    pub emoji_name: Option<String>,
    /// The id of the guild this sound is in.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "crate::message::ser_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// Whether this sound can be used.
    pub available: bool,
    /// The user who created this sound.
    #[serde(default)]
    pub user: Option<crate::interaction::User>,
}

// ───────────────────────── Voice Region Models ─────────────────────────

/// A voice region object (`GET /voice/regions`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct VoiceRegion {
    /// Unique ID for the region (e.g. "us-west", "brazil", "rotterdam").
    pub id: String,
    /// Name of the region.
    pub name: String,
    /// Whether this is a single-server voice region.
    pub custom: bool,
    /// Whether this is a deprecated voice region (avoid connecting to these).
    pub deprecated: bool,
    /// True for a single server that is closest to the current user's client.
    pub optimal: bool,
}

// ───────────────────────── SKU Subscription Models ─────────────────────────

/// Query parameters for `GET /skus/{sku_id}/subscriptions`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct GetSkuSubscriptions {
    /// List subscriptions before this ID.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub before: Option<Snowflake>,
    /// List subscriptions after this ID.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub after: Option<Snowflake>,
    /// Number of results to return (1-100). Defaults to 30.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u8>,
    /// Filter subscriptions by user ID.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub user_id: Option<Snowflake>,
}

/// Status of a subscription.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum SubscriptionStatus {
    /// Subscription is active and any entitlements are granted.
    Active = 0,
    /// Subscription is ending and will not renew at the end of the current period.
    Ending = 1,
    /// Subscription is inactive and entitlements are not granted.
    Inactive = 2,
}

impl SubscriptionStatus {
    /// Parse raw integer.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Active),
            1 => Some(Self::Ending),
            2 => Some(Self::Inactive),
            _ => None,
        }
    }
    /// As integer.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// A subscription to an application SKU (`GET /skus/{sku_id}/subscriptions/{subscription_id}`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Subscription {
    /// ID of the subscription.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// ID of the user who is subscribed.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,
    /// List of SKUs subscribed to.
    #[serde(deserialize_with = "de_vec_snowflake", serialize_with = "ser_vec_snowflake")]
    pub sku_ids: Vec<Snowflake>,
    /// List of entitlements granted for this subscription.
    #[serde(deserialize_with = "de_vec_snowflake", serialize_with = "ser_vec_snowflake")]
    pub entitlement_ids: Vec<Snowflake>,
    /// List of SKUs that this user will be subscribed to at renewal.
    #[serde(default, deserialize_with = "de_opt_vec_snowflake", serialize_with = "ser_opt_vec_snowflake")]
    pub renewal_sku_ids: Option<Vec<Snowflake>>,
    /// Start of the current subscription period (ISO8601).
    pub current_period_start: String,
    /// End of the current subscription period (ISO8601).
    pub current_period_end: String,
    /// Current status of the subscription.
    pub status: u8,
    /// When the subscription was canceled (ISO8601).
    #[serde(default)]
    pub canceled_at: Option<String>,
    /// ISO3166-1 alpha-2 country code of the payment source used.
    #[serde(default)]
    pub country: Option<String>,
}

// ───────────────────────── Gateway Models ─────────────────────────

/// Response returned by `GET /gateway`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Gateway {
    /// The WSS URL to connect to.
    pub url: String,
}

// ───────────────────────── Validation Errors ─────────────────────────

/// Validation error for miscellaneous endpoints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MiscError {
    /// SKU subscription limit must be between 1 and 100.
    BadSubscriptionLimit {
        /// Provided value.
        value: u8,
    },
}

impl fmt::Display for MiscError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadSubscriptionLimit { value } => {
                write!(f, "subscription limit {value} is out of bounds (must be 1-100)")
            }
        }
    }
}

impl std::error::Error for MiscError {}

impl From<MiscError> for Error {
    fn from(e: MiscError) -> Self {
        Error::Transport(format!("misc validation: {e}"))
    }
}

// ───────────────────────── REST Methods ─────────────────────────

impl<T: Transport> RestClient<T> {
    /// `GET /soundboard-default-sounds` – Returns a list of default soundboard sounds.
    pub async fn get_soundboard_default_sounds(&self) -> Result<Vec<SoundboardSound>, Error> {
        self.request("GET", "/soundboard-default-sounds", None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /voice/regions` – Returns a list of voice regions that can be used when creating servers.
    pub async fn list_voice_regions(&self) -> Result<Vec<VoiceRegion>, Error> {
        self.request("GET", "/voice/regions", None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /skus/{sku_id}/subscriptions` – Returns all subscriptions for a given SKU.
    pub async fn get_sku_subscriptions(
        &self,
        sku_id: Snowflake,
        query: &GetSkuSubscriptions,
    ) -> Result<Vec<Subscription>, Error> {
        if let Some(lim) = query.limit {
            if !(1..=100).contains(&lim) {
                return Err(MiscError::BadSubscriptionLimit { value: lim }.into());
            }
        }
        let qs = query_string(query);
        let path = format!("/skus/{sku_id}/subscriptions{qs}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /skus/{sku_id}/subscriptions/{subscription_id}` – Returns a subscription by ID for a given SKU.
    pub async fn get_sku_subscription(
        &self,
        sku_id: Snowflake,
        subscription_id: Snowflake,
    ) -> Result<Subscription, Error> {
        let path = format!("/skus/{sku_id}/subscriptions/{subscription_id}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /gateway` – Returns a valid WSS URL to connect to.
    pub async fn get_gateway(&self) -> Result<Gateway, Error> {
        self.request("GET", "/gateway", None)
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
    fn subscription_status_parsing() {
        assert_eq!(SubscriptionStatus::from_u8(0), Some(SubscriptionStatus::Active));
        assert_eq!(SubscriptionStatus::from_u8(1), Some(SubscriptionStatus::Ending));
        assert_eq!(SubscriptionStatus::from_u8(2), Some(SubscriptionStatus::Inactive));
        assert_eq!(SubscriptionStatus::from_u8(5), None);
    }

    #[tokio::test(start_paused = true)]
    async fn soundboard_and_voice_regions_routes() {
        let sound_json = json!([{
            "name": "airhorn",
            "sound_id": "12345",
            "volume": 0.8,
            "available": true
        }])
        .to_string();

        let region_json = json!([{
            "id": "us-west",
            "name": "US West",
            "custom": false,
            "deprecated": false,
            "optimal": true
        }])
        .to_string();

        let gateway_json = json!({
            "url": "wss://gateway.discord.gg"
        })
        .to_string();

        let c = client(vec![
            resp(200, &sound_json),
            resp(200, &region_json),
            resp(200, &gateway_json),
        ]);

        let sounds = c.get_soundboard_default_sounds().await.unwrap();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].name, "airhorn");
        assert_eq!(sounds[0].sound_id, sf(12345));

        let regions = c.list_voice_regions().await.unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].id, "us-west");
        assert!(regions[0].optimal);

        let gw = c.get_gateway().await.unwrap();
        assert_eq!(gw.url, "wss://gateway.discord.gg");

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].url, "https://discord.com/api/v10/soundboard-default-sounds");
        assert_eq!(seen[0].method, "GET");

        assert_eq!(seen[1].url, "https://discord.com/api/v10/voice/regions");
        assert_eq!(seen[1].method, "GET");

        assert_eq!(seen[2].url, "https://discord.com/api/v10/gateway");
        assert_eq!(seen[2].method, "GET");
    }

    #[tokio::test(start_paused = true)]
    async fn sku_subscriptions_routes_and_validation() {
        let sub_json = json!({
            "id": "555",
            "user_id": "666",
            "sku_ids": ["777"],
            "entitlement_ids": ["888"],
            "current_period_start": "2026-01-01T00:00:00Z",
            "current_period_end": "2026-02-01T00:00:00Z",
            "status": 0
        })
        .to_string();

        let list_json = json!([
            {
                "id": "555",
                "user_id": "666",
                "sku_ids": ["777"],
                "entitlement_ids": ["888"],
                "current_period_start": "2026-01-01T00:00:00Z",
                "current_period_end": "2026-02-01T00:00:00Z",
                "status": 0
            }
        ])
        .to_string();

        let c = client(vec![resp(200, &list_json), resp(200, &sub_json)]);

        let q = GetSkuSubscriptions {
            limit: Some(50),
            user_id: Some(sf(666)),
            ..Default::default()
        };
        let subs = c.get_sku_subscriptions(sf(777), &q).await.unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].id, sf(555));

        let sub = c.get_sku_subscription(sf(777), sf(555)).await.unwrap();
        assert_eq!(sub.id, sf(555));
        assert_eq!(sub.user_id, sf(666));

        {
            let seen = c.transport().seen.lock().unwrap();
            assert_eq!(
                seen[0].url,
                "https://discord.com/api/v10/skus/777/subscriptions?limit=50&user_id=666"
            );
            assert_eq!(seen[0].method, "GET");

            assert_eq!(seen[1].url, "https://discord.com/api/v10/skus/777/subscriptions/555");
            assert_eq!(seen[1].method, "GET");
        }

        // Validation: limit 0 or >100
        let bad_zero = GetSkuSubscriptions { limit: Some(0), ..Default::default() };
        assert!(c.get_sku_subscriptions(sf(777), &bad_zero).await.is_err());
        let bad_over = GetSkuSubscriptions { limit: Some(101), ..Default::default() };
        assert!(c.get_sku_subscriptions(sf(777), &bad_over).await.is_err());
    }
}
