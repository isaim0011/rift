//! OAuth2 endpoints.
//!
//! Discord OAuth2 endpoints:
//! - `GET /oauth2/@me`: get current authorization information
//! - `GET /oauth2/applications/@me`: get current application information
//! - `GET /oauth2/keys`: get public keys used for signing OAuth2 tokens
//! - `GET /oauth2/userinfo`: get OpenID Connect user info

use crate::application::Application;
use crate::client::RestClient;
use crate::error::Error;
use crate::interaction::User;
use crate::transport::Transport;
use serde::Deserialize;

/// Current OAuth2 authorization information.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OAuth2Authorization {
    /// The current application.
    pub application: Application,
    /// ISO-8601 timestamp when the grant expires.
    pub expires: String,
    /// Scopes granted to the application.
    pub scopes: Vec<String>,
    /// The user who authorized the application.
    #[serde(default)]
    pub user: Option<User>,
}

/// A cryptographic key used by Discord to sign tokens.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OAuth2Key {
    /// Key type (e.g. RSA).
    pub kty: String,
    /// Intended key usage (e.g. sig).
    #[serde(rename = "use")]
    pub use_: String,
    /// Key ID.
    pub kid: String,
    /// RSA modulus.
    pub n: String,
    /// RSA exponent.
    pub e: String,
    /// Algorithm (e.g. RS256).
    pub alg: String,
}

/// Response containing Discord's OAuth2 public keys.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OAuth2Keys {
    /// Array of public keys.
    pub keys: Vec<OAuth2Key>,
}

/// User info returned by OpenID Connect.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OpenIdUserInfo {
    /// Subject identifier (Discord user id).
    pub sub: String,
    /// User's email address if email scope was granted.
    #[serde(default)]
    pub email: Option<String>,
    /// Whether the user's email is verified.
    #[serde(default)]
    pub email_verified: Option<bool>,
    /// Preferred username.
    #[serde(default)]
    pub preferred_username: Option<String>,
    /// User's server or display nickname.
    #[serde(default)]
    pub nickname: Option<String>,
    /// Avatar URL.
    #[serde(default)]
    pub picture: Option<String>,
    /// User locale.
    #[serde(default)]
    pub locale: Option<String>,
}

impl<T: Transport> RestClient<T> {
    /// `GET /oauth2/@me`: returns the current authorization info.
    pub async fn get_current_authorization_info(&self) -> Result<OAuth2Authorization, Error> {
        self.request("GET", "/oauth2/@me", None).await?.expect_success()?.json()
    }

    /// `GET /oauth2/applications/@me`: returns the current application info.
    pub async fn get_current_oauth2_application(&self) -> Result<Application, Error> {
        self.request("GET", "/oauth2/applications/@me", None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /oauth2/keys`: returns public keys used for signing OAuth2 tokens.
    pub async fn get_oauth2_keys(&self) -> Result<OAuth2Keys, Error> {
        self.request("GET", "/oauth2/keys", None).await?.expect_success()?.json()
    }

    /// `GET /oauth2/userinfo`: returns OpenID Connect user info.
    pub async fn get_openid_userinfo(&self) -> Result<OpenIdUserInfo, Error> {
        self.request("GET", "/oauth2/userinfo", None).await?.expect_success()?.json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{client, resp};

    const SAMPLE_AUTH: &str = r#"{
        "application": {
            "id": "111",
            "name": "Rift App",
            "icon": null,
            "description": "App description"
        },
        "expires": "2026-12-31T23:59:59Z",
        "scopes": ["identify", "email"],
        "user": {
            "id": "222",
            "username": "bimo",
            "bot": false
        }
    }"#;

    const SAMPLE_APP: &str = r#"{
        "id": "111",
        "name": "Rift App",
        "icon": null,
        "description": "App description"
    }"#;

    const SAMPLE_KEYS: &str = r#"{
        "keys": [
            {
                "kty": "RSA",
                "use": "sig",
                "kid": "key_1",
                "n": "modulus",
                "e": "AQAB",
                "alg": "RS256"
            }
        ]
    }"#;

    const SAMPLE_USERINFO: &str = r#"{
        "sub": "222",
        "email": "user@example.com",
        "email_verified": true,
        "preferred_username": "bimo",
        "picture": "https://cdn.discordapp.com/avatars/222/abc.png",
        "locale": "en-US"
    }"#;

    #[tokio::test(start_paused = true)]
    async fn oauth2_routes_match_spec() {
        let c = client(vec![
            resp(200, SAMPLE_AUTH),
            resp(200, SAMPLE_APP),
            resp(200, SAMPLE_KEYS),
            resp(200, SAMPLE_USERINFO),
        ]);

        let auth: OAuth2Authorization = c.get_current_authorization_info().await.unwrap();
        assert_eq!(auth.scopes, vec!["identify", "email"]);

        let app: Application = c.get_current_oauth2_application().await.unwrap();
        assert_eq!(app.name, "Rift App");

        let keys: OAuth2Keys = c.get_oauth2_keys().await.unwrap();
        assert_eq!(keys.keys.len(), 1);
        assert_eq!(keys.keys[0].kid, "key_1");

        let userinfo: OpenIdUserInfo = c.get_openid_userinfo().await.unwrap();
        assert_eq!(userinfo.sub, "222");
        assert_eq!(userinfo.preferred_username.as_deref(), Some("bimo"));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen.len(), 4);
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/oauth2/@me");
        assert_eq!(seen[1].method, "GET");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/oauth2/applications/@me");
        assert_eq!(seen[2].method, "GET");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/oauth2/keys");
        assert_eq!(seen[3].method, "GET");
        assert_eq!(seen[3].url, "https://discord.com/api/v10/oauth2/userinfo");
    }
}
