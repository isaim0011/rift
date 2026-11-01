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
        
        let mut seg_stack = [""; 16];
        let mut seg_vec = Vec::new();
        let mut seg_count = 0;
        
        for s in path.split('/') {
            if !s.is_empty() {
                if seg_count < 16 {
                    seg_stack[seg_count] = s;
                } else if seg_count == 16 {
                    seg_vec.extend_from_slice(&seg_stack);
                    seg_vec.push(s);
                } else {
                    seg_vec.push(s);
                }
                seg_count += 1;
            }
        }
        let segs: &[&str] = if seg_count <= 16 {
            &seg_stack[..seg_count]
        } else {
            &seg_vec
        };

        // Interaction endpoints are exempt from the global limit and carry a
        // per-interaction token, which must not leak into the route key.
        if segs.first() == Some(&"interactions") {
            let mut template = String::with_capacity(32 + path.len());
            template.push_str("/interactions/:id/:token");
            for s in segs.iter().skip(3) {
                template.push('/');
                template.push_str(s);
            }
            let mut id = String::with_capacity(method.len() + 1 + template.len());
            for b in method.bytes() {
                id.push(b.to_ascii_uppercase() as char);
            }
            id.push(' ');
            id.push_str(&template);
            return Self {
                id,
                template,
                major: "interactions".to_owned(),
                webhook: None,
                global_exempt: true,
                untracked: true,
            };
        }

        let mut template = String::with_capacity(path.len() + 16);
        let mut major = String::new();
        let mut webhook = None;
        let mut i = 0;
        while i < segs.len() {
            let s = segs[i];
            let prev = if i > 0 { segs[i - 1] } else { "" };
            template.push('/');
            if is_snowflake(s) && (prev == "channels" || prev == "guilds") {
                if major.is_empty() {
                    major.reserve(prev.len() + 1 + s.len());
                    major.push_str(prev);
                    major.push('/');
                    major.push_str(s);
                }
                template.push_str(":major");
            } else if is_snowflake(s) && prev == "webhooks" {
                let token = segs.get(i + 1).copied();
                let mut m = String::with_capacity(9 + s.len() + token.map_or(0, |t| 1 + t.len()));
                m.push_str("webhooks/");
                m.push_str(s);
                if let Some(tok) = token {
                    m.push('/');
                    m.push_str(tok);
                    i += 1;
                }
                if major.is_empty() {
                    major = m.clone();
                }
                webhook = Some(m);
                template.push_str(":major");
            } else if is_snowflake(s) {
                template.push_str(":id");
            } else if prev == "reactions" {
                template.push_str(":emoji");
            } else if prev == "invites" {
                template.push_str(":code");
            } else {
                template.push_str(s);
            }
            i += 1;
        }

        let mut id = String::with_capacity(method.len() + 1 + template.len());
        if method.bytes().all(|b| b.is_ascii_uppercase()) {
            id.push_str(method);
        } else {
            for b in method.bytes() {
                id.push(b.to_ascii_uppercase() as char);
            }
        }
        id.push(' ');
        id.push_str(&template);

        Self {
            id,
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
