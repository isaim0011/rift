//! Monetization models: entitlements and subscriptions.

use crate::serde_util::{
    de_opt_snowflake, de_opt_vec_snowflake, de_snowflake, de_vec_snowflake, ser_opt_snowflake,
    ser_opt_vec_snowflake, ser_snowflake, ser_vec_snowflake,
};
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};

/// An entitlement granting a user or guild access to a SKU.
///
/// See <https://discord.com/developers/docs/resources/entitlement#entitlement-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entitlement {
    /// ID of the entitlement.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// ID of the SKU.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub sku_id: Snowflake,

    /// ID of the parent application.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub application_id: Snowflake,

    /// ID of the user that is granted access to the entitlement's SKU.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub user_id: Option<Snowflake>,

    /// Type of entitlement.
    #[serde(rename = "type")]
    pub kind: u8,

    /// Entitlement was deleted.
    #[serde(default)]
    pub deleted: bool,

    /// Start date at which the entitlement is valid.
    #[serde(default)]
    pub starts_at: Option<String>,

    /// Date at which the entitlement is no longer valid.
    #[serde(default)]
    pub ends_at: Option<String>,

    /// ID of the guild that is granted access to the entitlement's SKU.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// For consumable items, whether or not the entitlement has been consumed.
    #[serde(default)]
    pub consumed: Option<bool>,

    /// ID of the subscription that the entitlement is part of, if applicable.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub subscription_id: Option<Snowflake>,
}

/// A recurring monetization subscription.
///
/// See <https://discord.com/developers/docs/resources/subscription#subscription-structure>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    /// ID of the subscription.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// ID of the user who is subscribed.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// List of SKUs subscribed to.
    #[serde(
        deserialize_with = "de_vec_snowflake",
        serialize_with = "ser_vec_snowflake"
    )]
    pub sku_ids: Vec<Snowflake>,

    /// List of entitlements granted for this subscription.
    #[serde(
        deserialize_with = "de_vec_snowflake",
        serialize_with = "ser_vec_snowflake"
    )]
    pub entitlement_ids: Vec<Snowflake>,

    /// List of SKUs that this user will be subscribed to at renewal.
    #[serde(
        default,
        deserialize_with = "de_opt_vec_snowflake",
        serialize_with = "ser_opt_vec_snowflake"
    )]
    pub renewal_sku_ids: Option<Vec<Snowflake>>,

    /// Start of the current subscription period.
    pub current_period_start: String,

    /// End of the current subscription period.
    pub current_period_end: String,

    /// Current status of the subscription (0=Active, 1=Ending, 2=Inactive).
    pub status: u8,

    /// When the subscription was canceled (if applicable).
    #[serde(default)]
    pub canceled_at: Option<String>,

    /// Country code of the payment source used.
    #[serde(default)]
    pub country: Option<String>,
}
