/// A request route, normalized for rate-limit bookkeeping.
///
/// Top-level resource ids (channel, guild, webhook) become the *major*
/// parameter; every other snowflake becomes `:id`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub(crate) id: String,
    pub(crate) template: String,
    pub(crate) major: String,
    pub(crate) webhook: Option<String>,
    pub(crate) global_exempt: bool,
    /// Not bucket-tracked at all (interaction callbacks: time-critical, one call per token).
    pub(crate) untracked: bool,
}

fn is_snowflake(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

impl Route {
    /// Normalize `method` + `path` (a query string is ignored).
    pub fn new(method: &str, path: &str) -> Self {
        let path = path.split('?').next().unwrap_or("");
        let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // Interaction endpoints are exempt from the global limit and carry a
        // per-interaction token, which must not leak into the route key.
        if segs.first() == Some(&"interactions") {
            let rest: String = segs.iter().skip(3).map(|s| format!("/{s}")).collect();
            let template = format!("/interactions/:id/:token{rest}");
            let method = method.to_ascii_uppercase();
            return Self {
                id: format!("{method} {template}"),
                template,
                major: "interactions".to_owned(),
                webhook: None,
                global_exempt: true,
                untracked: true,
            };
        }

        let mut out: Vec<String> = Vec::with_capacity(segs.len());
        let mut major = String::new();
        let mut webhook = None;
        let mut i = 0;
        while i < segs.len() {
            let s = segs[i];
            let prev = if i > 0 { segs[i - 1] } else { "" };
            if is_snowflake(s) && (prev == "channels" || prev == "guilds") {
                if major.is_empty() {
                    major = format!("{prev}/{s}");
                }
                out.push(":major".to_owned());
            } else if is_snowflake(s) && prev == "webhooks" {
                let mut m = format!("webhooks/{s}");
                if let Some(token) = segs.get(i + 1) {
                    m.push('/');
                    m.push_str(token);
                    i += 1; // the token is part of the major key, not the template
                }
                if major.is_empty() {
                    major = m.clone();
                }
                webhook = Some(m);
                out.push(":major".to_owned());
            } else if is_snowflake(s) {
                out.push(":id".to_owned());
            } else if prev == "reactions" {
                out.push(":emoji".to_owned());
            } else if prev == "invites" {
                out.push(":code".to_owned());
            } else {
                out.push(s.to_owned());
            }
            i += 1;
        }

        let template = format!("/{}", out.join("/"));
        let method = method.to_ascii_uppercase();
        Self {
            id: format!("{method} {template}"),
            template,
            major,
            webhook,
            global_exempt: false,
            untracked: false,
        }
    }

    /// Normalized path with ids replaced, e.g. `/channels/:major/messages/:id`.
    pub fn template(&self) -> &str {
        &self.template
    }

    /// The top-level resource this request is limited under, e.g. `channels/123`.
    pub fn major(&self) -> &str {
        &self.major
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_route_keeps_channel_as_major() {
        let r = Route::new("get", "/channels/123/messages/456?limit=5");
        assert_eq!(r.template(), "/channels/:major/messages/:id");
        assert_eq!(r.major(), "channels/123");
        assert_eq!(r.id, "GET /channels/:major/messages/:id");
    }

    #[test]
    fn different_channels_share_a_template_but_not_a_major() {
        let a = Route::new("POST", "/channels/1/messages");
        let b = Route::new("POST", "/channels/2/messages");
        assert_eq!(a.id, b.id);
        assert_ne!(a.major(), b.major());
    }

    #[test]
    fn guild_and_unrelated_routes() {
        assert_eq!(Route::new("GET", "/guilds/9/members/5").major(), "guilds/9");
        let me = Route::new("GET", "/users/@me");
        assert_eq!(me.template(), "/users/@me");
        assert_eq!(me.major(), "");
    }

    #[test]
    fn webhook_major_includes_token_but_template_does_not() {
        let r = Route::new("POST", "/webhooks/10/secrettoken/messages/5");
        assert_eq!(r.major(), "webhooks/10/secrettoken");
        assert_eq!(r.template(), "/webhooks/:major/messages/:id");
        assert_eq!(r.webhook.as_deref(), Some("webhooks/10/secrettoken"));
    }

    #[test]
    fn interactions_are_global_exempt_and_token_free() {
        let r = Route::new("POST", "/interactions/77/abcTOKEN/callback");
        assert!(r.global_exempt);
        assert_eq!(r.template(), "/interactions/:id/:token/callback");
    }

    #[test]
    fn only_interaction_callbacks_are_untracked() {
        assert!(Route::new("POST", "/interactions/1/tok/callback").untracked);
        assert!(!Route::new("POST", "/webhooks/1/tok").untracked);
        assert!(!Route::new("POST", "/channels/1/messages").untracked);
    }

    #[test]
    fn reaction_emoji_is_collapsed() {
        let r = Route::new("PUT", "/channels/1/messages/2/reactions/%F0%9F%91%8D/@me");
        assert_eq!(r.template(), "/channels/:major/messages/:id/reactions/:emoji/@me");
    }
}
