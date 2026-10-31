//! Serde serialization and deserialization helpers for Discord primitives.

use rift_core::{Permissions, Snowflake};
use serde::de::{self, Deserializer, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;

struct SnowflakeVisitor;

impl<'de> Visitor<'de> for SnowflakeVisitor {
    type Value = Snowflake;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a snowflake as a string or integer")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        v.parse::<Snowflake>().map_err(E::custom)
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Snowflake::new(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        if v >= 0 {
            Ok(Snowflake::new(v as u64))
        } else {
            Err(E::custom("snowflake cannot be negative"))
        }
    }
}

/// Deserialize a [`Snowflake`] from a decimal string or integer.
pub fn de_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Snowflake, D::Error> {
    d.deserialize_any(SnowflakeVisitor)
}

/// Serialize a [`Snowflake`] as a string.
pub fn ser_snowflake<S: Serializer>(s: &Snowflake, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&s.to_string())
}

struct OptSnowflakeVisitor;

impl<'de> Visitor<'de> for OptSnowflakeVisitor {
    type Value = Option<Snowflake>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an optional snowflake as a string, integer, or null")
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(SnowflakeVisitor).map(Some)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v.is_empty() {
            Ok(None)
        } else {
            v.parse::<Snowflake>().map(Some).map_err(E::custom)
        }
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Some(Snowflake::new(v)))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        if v >= 0 {
            Ok(Some(Snowflake::new(v as u64)))
        } else {
            Err(E::custom("snowflake cannot be negative"))
        }
    }
}

/// Deserialize an optional [`Snowflake`] from a string, integer, or null.
pub fn de_opt_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Snowflake>, D::Error> {
    d.deserialize_any(OptSnowflakeVisitor)
}

/// Serialize an optional [`Snowflake`] as a string or null.
pub fn ser_opt_snowflake<S: Serializer>(
    s: &Option<Snowflake>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match s {
        Some(snowflake) => serializer.serialize_some(&snowflake.to_string()),
        None => serializer.serialize_none(),
    }
}

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
struct SnowflakeHelper(
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")] Snowflake,
);

/// Deserialize a vector of [`Snowflake`]s.
pub fn de_vec_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Snowflake>, D::Error> {
    let helpers = Vec::<SnowflakeHelper>::deserialize(d)?;
    Ok(helpers.into_iter().map(|h| h.0).collect())
}

/// Serialize a vector of [`Snowflake`]s as strings.
pub fn ser_vec_snowflake<S: Serializer>(
    list: &[Snowflake],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(list.len()))?;
    for item in list {
        seq.serialize_element(&item.to_string())?;
    }
    seq.end()
}

/// Deserialize an optional vector of [`Snowflake`]s.
pub fn de_opt_vec_snowflake<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<Vec<Snowflake>>, D::Error> {
    let opt = Option::<Vec<SnowflakeHelper>>::deserialize(d)?;
    Ok(opt.map(|helpers| helpers.into_iter().map(|h| h.0).collect()))
}

/// Serialize an optional vector of [`Snowflake`]s.
pub fn ser_opt_vec_snowflake<S: Serializer>(
    list: &Option<Vec<Snowflake>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match list {
        Some(vec) => ser_vec_snowflake(vec, serializer),
        None => serializer.serialize_none(),
    }
}

struct PermissionsVisitor;

impl<'de> Visitor<'de> for PermissionsVisitor {
    type Value = Permissions;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("permissions as a decimal string or integer")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let bits = v.parse::<u64>().map_err(E::custom)?;
        Ok(Permissions::from_bits_retain(bits))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Permissions::from_bits_retain(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        if v >= 0 {
            Ok(Permissions::from_bits_retain(v as u64))
        } else {
            Err(E::custom("permissions cannot be negative"))
        }
    }
}

/// Deserialize [`Permissions`] from a decimal string or integer.
pub fn de_permissions<'de, D: Deserializer<'de>>(d: D) -> Result<Permissions, D::Error> {
    d.deserialize_any(PermissionsVisitor)
}

/// Serialize [`Permissions`] as a decimal string.
pub fn ser_permissions<S: Serializer>(
    p: &Permissions,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&p.bits().to_string())
}

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
struct PermissionsHelper(
    #[serde(deserialize_with = "de_permissions", serialize_with = "ser_permissions")] Permissions,
);

/// Deserialize optional [`Permissions`].
pub fn de_opt_permissions<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<Permissions>, D::Error> {
    let opt = Option::<PermissionsHelper>::deserialize(d)?;
    Ok(opt.map(|h| h.0))
}

/// Serialize optional [`Permissions`].
pub fn ser_opt_permissions<S: Serializer>(
    p: &Option<Permissions>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match p {
        Some(permissions) => ser_permissions(permissions, serializer),
        None => serializer.serialize_none(),
    }
}
