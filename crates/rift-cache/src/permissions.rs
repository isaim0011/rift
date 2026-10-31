//! Permission resolution algorithms and helpers matching Discord's permission hierarchy.

use rift_core::permissions::{self as core_perms, Overwrite};
use rift_core::Permissions;
use std::time::{SystemTime, UNIX_EPOCH};

/// Guild-level base permissions. Guild owners and administrators receive [`Permissions::ALL`].
pub fn compute_base_permissions(
    is_owner: bool,
    everyone: Permissions,
    member_roles: impl IntoIterator<Item = Permissions>,
) -> Permissions {
    core_perms::compute_base(is_owner, everyone, member_roles)
}

/// Channel-level permission overwrites.
///
/// Overwrites are applied in order:
/// 1. `@everyone` overwrite
/// 2. Member role overwrites (denies combined, allows combined)
/// 3. Member-specific overwrite
pub fn compute_overwrites(
    base: Permissions,
    everyone: Option<Overwrite>,
    role_overwrites: impl IntoIterator<Item = Overwrite>,
    member: Option<Overwrite>,
) -> Permissions {
    core_perms::compute_overwrites(base, everyone, role_overwrites, member)
}

/// Applies implicit permission rules for text channels.
///
/// - If `VIEW_CHANNEL` is missing, all permissions are denied.
/// - If `SEND_MESSAGES` is missing, dependent permissions (`EMBED_LINKS`, `ATTACH_FILES`, etc.) are stripped.
pub fn apply_implicit_permissions(p: Permissions) -> Permissions {
    core_perms::apply_implicit_text(p)
}

/// Restricts timed-out members to `VIEW_CHANNEL` and `READ_MESSAGE_HISTORY`.
/// Guild owners and administrators are exempt.
pub fn apply_timeout(p: Permissions, is_owner: bool) -> Permissions {
    core_perms::apply_timeout(p, is_owner)
}

/// Checks whether an ISO 8601 timestamp string indicates an active timeout in the future.
pub fn is_timed_out(communication_disabled_until: Option<&str>) -> bool {
    let s = match communication_disabled_until {
        Some(s) if !s.is_empty() => s,
        _ => return false,
    };

    let target_secs = match parse_iso8601_to_unix(s) {
        Some(ts) => ts,
        None => return false,
    };

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    target_secs > now_secs
}

/// Parses an ISO 8601 string (e.g. `2026-11-01T12:00:00Z` or `2026-11-01T12:00:00+00:00`)
/// into Unix epoch seconds.
pub fn parse_iso8601_to_unix(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }

    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: u64 = s.get(5..7)?.parse().ok()?;
    let day: u64 = s.get(8..10)?.parse().ok()?;
    let hour: u64 = s.get(11..13)?.parse().ok()?;
    let minute: u64 = s.get(14..16)?.parse().ok()?;
    let second: u64 = s.get(17..19)?.parse().ok()?;

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let days = days_from_civil(year, month, day);
    let mut secs = days * 86400 + (hour * 3600 + minute * 60 + second) as i64;

    // Parse potential timezone offset
    let rem = &s[19..];
    let rem = if let Some(dot_idx) = rem.find(['+', '-', 'Z']) {
        &rem[dot_idx..]
    } else {
        rem
    };

    if !rem.is_empty() && rem != "Z" {
        let sign = if rem.starts_with('+') {
            -1
        } else if rem.starts_with('-') {
            1
        } else {
            0
        };

        if sign != 0 && rem.len() >= 6 {
            let tz_hour: u64 = rem.get(1..3)?.parse().ok()?;
            let tz_min: u64 = rem.get(4..6)?.parse().ok()?;
            let tz_offset_secs = (tz_hour * 3600 + tz_min * 60) as i64;
            secs += sign * tz_offset_secs;
        }
    }

    if secs < 0 {
        None
    } else {
        Some(secs as u64)
    }
}

/// Howard Hinnant's algorithm for converting Gregorian date to days since 1970-01-01.
fn days_from_civil(mut y: i64, m: u64, d: u64) -> i64 {
    y -= if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + (doe as i64) - 719468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_iso8601() {
        assert_eq!(
            parse_iso8601_to_unix("1970-01-01T00:00:00Z"),
            Some(0)
        );
        assert_eq!(
            parse_iso8601_to_unix("2026-10-31T12:00:00Z"),
            Some(1793448000)
        );
        assert_eq!(
            parse_iso8601_to_unix("2026-10-31T12:00:00.123456+00:00"),
            Some(1793448000)
        );
    }
}
