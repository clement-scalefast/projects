//! Signs users in through Keycloak with the OIDC authorization code flow. The
//! browser goes to Keycloak to sign in and comes back with a code. The app
//! exchanges the code for an ID token that names the user.

use std::env;

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use jiff::Timestamp;
use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use topcoat::{
    Result,
    context::{Cx, context},
    cookie::{Cookie, Cookies, SameSite, cookies, time::Duration},
    router::{error::internal_server_error, request::headers},
};

use super::user::Role;

/// Holds the state, nonce, and code verifier of a sign-in between the redirect to
/// Keycloak and the return to the app.
const PENDING_COOKIE: &str = "sign_in";

/// How long a user has to finish signing in at Keycloak.
const PENDING_MINUTES: i64 = 10;

/// The path Keycloak sends the browser back to. It must be an allowed redirect
/// URI of the Keycloak client.
const CALLBACK_PATH: &str = "/login/callback";

/// The path Keycloak sends the browser back to after signing out.
const SIGNED_OUT_PATH: &str = "/login";

/// The connection to a Keycloak realm, read from the environment at startup.
#[derive(Debug, Clone)]
pub struct Keycloak {
    /// The realm's URL as the browser reaches it. Keycloak also uses it as the
    /// issuer of its tokens.
    issuer: String,
    /// The realm's URL as the app reaches it. It differs from the issuer when the
    /// app runs in a container next to Keycloak.
    backchannel: String,
    client_id: String,
    client_secret: String,
    /// The app's public URL. Without it, the app uses the host of each request.
    app_url: Option<String>,
    http: reqwest::Client,
}

impl Keycloak {
    /// Reads the settings from the environment. The defaults match the Keycloak
    /// that `docker compose up keycloak` starts.
    pub fn from_env() -> Self {
        let var = |name: &str, default: &str| {
            let value = env::var(name).unwrap_or_else(|_| default.to_owned());
            value.trim_end_matches('/').to_owned()
        };
        let url = var("KEYCLOAK_URL", "http://localhost:8080");
        let internal_url = var("KEYCLOAK_INTERNAL_URL", &url);
        let realm = var("KEYCLOAK_REALM", "projects");

        Self {
            issuer: format!("{url}/realms/{realm}"),
            backchannel: format!("{internal_url}/realms/{realm}"),
            client_id: var("KEYCLOAK_CLIENT_ID", "projects"),
            client_secret: var("KEYCLOAK_CLIENT_SECRET", "projects-dev-secret"),
            app_url: env::var("APP_URL")
                .ok()
                .map(|url| url.trim_end_matches('/').to_owned()),
            http: reqwest::Client::new(),
        }
    }

    /// Builds the app's public URL for a path.
    fn app_url(&self, cx: &Cx, path: &str) -> String {
        if let Some(app_url) = &self.app_url {
            return format!("{app_url}{path}");
        }
        let host = headers(cx)
            .get("host")
            .and_then(|host| host.to_str().ok())
            .unwrap_or("localhost:3000");
        format!("http://{host}{path}")
    }
}

fn keycloak(cx: &Cx) -> &Keycloak {
    context::<Keycloak>(cx)
}

/// The person Keycloak signed in.
#[derive(Debug)]
pub struct Identity {
    /// Keycloak's ID of the user, which stays the same when the email changes.
    pub subject: String,
    pub email: String,
    pub name: String,
    pub role: Role,
    /// Kept to sign the user out of Keycloak later.
    pub id_token: String,
}

/// Starts a sign-in and returns the Keycloak URL to send the browser to.
pub fn start_sign_in(cx: &Cx) -> Result<String> {
    let keycloak = keycloak(cx);
    let state = random_token();
    let nonce = random_token();
    let verifier = random_token();
    let challenge = BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(&verifier));

    // The callback is a navigation from Keycloak's site, which `SameSite=Lax` allows.
    cookies(cx).add(
        Cookie::build((PENDING_COOKIE, format!("{state}.{nonce}.{verifier}")))
            .path("/")
            .max_age(Duration::minutes(PENDING_MINUTES))
            .http_only(true)
            .same_site(SameSite::Lax),
    );

    let url = Url::parse_with_params(
        &format!("{}/protocol/openid-connect/auth", keycloak.issuer),
        [
            ("response_type", "code"),
            ("client_id", &keycloak.client_id),
            ("redirect_uri", &keycloak.app_url(cx, CALLBACK_PATH)),
            ("scope", "openid profile email"),
            ("state", &state),
            ("nonce", &nonce),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
        ],
    )
    .map_err(internal_server_error)?;
    Ok(url.into())
}

/// Finishes a sign-in with the code and state that Keycloak sent back. Returns
/// `None` if they do not belong to a sign-in this browser started, or if Keycloak
/// rejects them.
pub async fn finish_sign_in(cx: &Cx, code: &str, state: &str) -> Result<Option<Identity>> {
    let keycloak = keycloak(cx);
    let jar = cookies(cx);
    let Some(pending) = jar.get(PENDING_COOKIE) else {
        return Ok(None);
    };
    jar.remove(Cookie::build((PENDING_COOKIE, "")).path("/"));

    let mut parts = pending.value().split('.');
    let (Some(expected_state), Some(nonce), Some(verifier), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Ok(None);
    };
    if state != expected_state {
        return Ok(None);
    }

    let response = keycloak
        .http
        .post(format!(
            "{}/protocol/openid-connect/token",
            keycloak.backchannel
        ))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", &keycloak.client_id),
            ("client_secret", &keycloak.client_secret),
            ("code", code),
            ("redirect_uri", &keycloak.app_url(cx, CALLBACK_PATH)),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(internal_server_error)?;
    // Keycloak answers 400 for a code that is used, expired, or from another client.
    if response.status().is_client_error() {
        return Ok(None);
    }
    let tokens = response
        .error_for_status()
        .map_err(internal_server_error)?
        .json::<TokenResponse>()
        .await
        .map_err(internal_server_error)?;

    let Some(claims) = Claims::decode(&tokens.id_token) else {
        return Ok(None);
    };
    let valid = claims.iss == keycloak.issuer
        && claims.aud.contains(&keycloak.client_id)
        && claims.exp > Timestamp::now().as_second()
        && claims.nonce.as_deref() == Some(nonce);
    if !valid {
        return Ok(None);
    }
    let Some(email) = claims.email else {
        return Ok(None);
    };

    Ok(Some(Identity {
        role: role(&claims.roles),
        name: claims
            .name
            .or(claims.preferred_username)
            .unwrap_or_else(|| email.clone()),
        email: email.trim().to_lowercase(),
        subject: claims.sub,
        id_token: tokens.id_token,
    }))
}

/// Returns the Keycloak URL that ends the user's Keycloak session and then sends
/// the browser back to the sign-in page. Keycloak asks the user to confirm if
/// there is no ID token.
pub fn sign_out_url(cx: &Cx, id_token: Option<&str>) -> Result<String> {
    let keycloak = keycloak(cx);
    let mut params = vec![
        ("client_id", keycloak.client_id.clone()),
        (
            "post_logout_redirect_uri",
            keycloak.app_url(cx, SIGNED_OUT_PATH),
        ),
    ];
    if let Some(id_token) = id_token {
        params.push(("id_token_hint", id_token.to_owned()));
    }
    let url = Url::parse_with_params(
        &format!("{}/protocol/openid-connect/logout", keycloak.issuer),
        params,
    )
    .map_err(internal_server_error)?;
    Ok(url.into())
}

/// Picks the highest role among the user's Keycloak realm roles. A user without
/// one of the app's roles can only read.
fn role(roles: &[String]) -> Role {
    let has = |name: &str| roles.iter().any(|role| role == name);
    if has("admin") {
        Role::Admin
    } else if has("staff") {
        Role::Staff
    } else {
        Role::Viewer
    }
}

/// Returns 32 random bytes in URL-safe Base64.
fn random_token() -> String {
    BASE64_URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: String,
}

/// The ID token claims the app reads.
#[derive(Debug, Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: Audience,
    /// The expiry time in seconds since the Unix epoch.
    exp: i64,
    nonce: Option<String>,
    email: Option<String>,
    name: Option<String>,
    preferred_username: Option<String>,
    /// The realm roles, added by a mapper on the Keycloak client.
    #[serde(default)]
    roles: Vec<String>,
}

impl Claims {
    /// Reads the claims of an ID token without checking its signature. That is
    /// safe only for a token the app received from Keycloak's token endpoint
    /// itself, never for one that came through the browser.
    fn decode(id_token: &str) -> Option<Self> {
        let payload = id_token.split('.').nth(1)?;
        let payload = BASE64_URL_SAFE_NO_PAD.decode(payload).ok()?;
        serde_json::from_slice(&payload).ok()
    }
}

/// The clients a token is for. Keycloak sends one name or a list.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn contains(&self, client_id: &str) -> bool {
        match self {
            Self::One(audience) => audience == client_id,
            Self::Many(audiences) => audiences.iter().any(|audience| audience == client_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    #[test]
    fn picks_the_highest_role() {
        assert_eq!(role(&roles(&["viewer", "admin", "staff"])), Role::Admin);
        assert_eq!(role(&roles(&["offline_access", "staff"])), Role::Staff);
        assert_eq!(role(&roles(&["default-roles-projects"])), Role::Viewer);
        assert_eq!(role(&[]), Role::Viewer);
    }

    #[test]
    fn decodes_claims() {
        let payload = r#"{"iss":"http://localhost:8080/realms/projects","sub":"42",
            "aud":"projects","exp":1,"nonce":"n","email":"ada@projects.test",
            "roles":["admin"]}"#;
        let token = format!(
            "header.{}.signature",
            BASE64_URL_SAFE_NO_PAD.encode(payload)
        );

        let claims = Claims::decode(&token).unwrap();
        assert_eq!(claims.sub, "42");
        assert!(claims.aud.contains("projects"));
        assert_eq!(claims.roles, ["admin"]);
        assert!(Claims::decode("not a token").is_none());
    }

    #[test]
    fn accepts_a_list_of_audiences() {
        let audience: Audience = serde_json::from_str(r#"["account", "projects"]"#).unwrap();
        assert!(audience.contains("projects"));
        assert!(!audience.contains("other"));
    }
}
