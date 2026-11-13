use topcoat::{Result, context::Cx};

use super::keycloak::Identity;

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

    /// Keycloak's ID of the user. Empty until the user signs in for the first time.
    #[unique]
    subject: Option<String>,

    pub role: Role,
}

impl User {
    /// Returns the team member for a person Keycloak signed in, with the name,
    /// email, and role that Keycloak reports. A person who signs in for the first
    /// time takes over the member with the same email, or becomes a new member.
    pub async fn from_identity(cx: &Cx, identity: &Identity) -> Result<User> {
        let mut db = db(cx);
        let subject = Some(identity.subject.clone());

        let mut user = User::filter_by_subject(subject.clone())
            .first()
            .exec(&mut db)
            .await?;
        if user.is_none() {
            user = User::filter_by_email(&identity.email)
                .first()
                .exec(&mut db)
                .await?;
        }

        let Some(mut user) = user else {
            let user = toasty::create!(User {
                name: &identity.name,
                email: &identity.email,
                subject,
                role: identity.role,
            })
            .exec(&mut db)
            .await?;
            return Ok(user);
        };

        user.update()
            .name(&identity.name)
            .email(&identity.email)
            .subject(subject)
            .role(identity.role)
            .exec(&mut db)
            .await?;
        Ok(user)
    }

    /// Adds a team member who has not signed in yet.
    pub async fn invite(
        db: &mut toasty::Db,
        name: &str,
        email: &str,
        role: Role,
    ) -> toasty::Result<User> {
        toasty::create!(User {
            name,
            email,
            subject: None::<String>,
            role,
        })
        .exec(db)
        .await
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
