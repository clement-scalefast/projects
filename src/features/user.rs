use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use topcoat::{Result, context::Cx, router::error::internal_server_error};

use crate::db::db;

/// A team member who can sign in to the workspace.
#[derive(Debug, toasty::Model)]
pub struct User {
    #[key]
    #[auto]
    pub id: u64,

    pub name: String,

    #[unique]
    pub email: String,

    /// Password hash in Argon2 PHC format.
    password_hash: String,

    pub role: Role,
}

impl User {
    /// Checks the credentials and returns the user, or `None` if they do not match.
    pub async fn authenticate(cx: &Cx, email: &str, password: String) -> Result<Option<User>> {
        let user = User::filter_by_email(email.trim().to_lowercase())
            .first()
            .exec(&mut db(cx))
            .await?;

        let Some(user) = user else {
            return Ok(None);
        };

        let password_hash = user.password_hash.clone();
        let matches =
            tokio::task::spawn_blocking(move || verify_password(&password, &password_hash))
                .await
                .map_err(internal_server_error)?;

        Ok(matches.then_some(user))
    }

    pub async fn find(cx: &Cx, id: u64) -> Result<Option<User>> {
        let user = User::filter_by_id(id).first().exec(&mut db(cx)).await?;
        Ok(user)
    }

    /// Loads every team member, sorted by name.
    pub async fn list(cx: &Cx) -> Result<Vec<User>> {
        let users = User::all()
            .order_by(User::fields().name().asc())
            .exec(&mut db(cx))
            .await?;
        Ok(users)
    }

    pub fn initials(&self) -> String {
        self.name
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum Role {
    /// Manages projects and everything members can do.
    Admin,
    /// A regular member. Shown as "Member"; the stored label stays `staff`.
    Staff,
    /// Reads everything but changes nothing.
    Viewer,
}

impl Role {
    /// Whether the role may create and change issues and comments.
    pub fn can_edit(self) -> bool {
        matches!(self, Self::Admin | Self::Staff)
    }

    /// Whether the role may create projects and change their settings.
    pub fn can_manage(self) -> bool {
        matches!(self, Self::Admin)
    }
}

/// Creates a salted Argon2 password hash. This is CPU-intensive; use
/// [`tokio::task::spawn_blocking`] when calling it from async code.
pub fn hash_password(password: &str) -> String {
    Argon2::default()
        .hash_password(password.as_bytes())
        .expect("hashing a password with the default parameters succeeds")
        .to_string()
}

fn verify_password(password: &str, password_hash: &str) -> bool {
    PasswordHash::new(password_hash).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}
