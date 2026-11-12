use jiff::Timestamp;
use serde::Deserialize;
use topcoat::{Result, context::Cx};

use super::{
    issue::{Issue, IssueStatus},
    user::User,
};
use crate::db::db;

/// Maximum name length in characters.
const MAX_NAME: usize = 60;

/// Key length limits in characters.
const MIN_CODE: usize = 2;
const MAX_CODE: usize = 5;

/// Maximum description length in characters.
const MAX_DESCRIPTION: usize = 280;

#[derive(Debug, toasty::Model)]
pub struct Project {
    #[key]
    #[auto]
    pub id: u64,

    pub name: String,

    /// The project key that prefixes issue keys, such as `WEB` in `WEB-12`.
    #[unique]
    pub code: String,

    pub description: String,

    #[index]
    pub lead_id: u64,

    #[belongs_to]
    pub lead: toasty::Deferred<User>,

    #[has_many]
    pub issues: toasty::Deferred<Vec<Issue>>,

    pub created_at: Timestamp,
}

impl Project {
    /// Loads a project and its lead.
    pub async fn find(cx: &Cx, id: u64) -> Result<Option<Project>> {
        let project = Project::filter_by_id(id)
            .include(Project::fields().lead())
            .first()
            .exec(&mut db(cx))
            .await?;
        Ok(project)
    }

    /// Loads every project with its lead and issues, sorted by name.
    pub async fn list(cx: &Cx) -> Result<Vec<Project>> {
        let projects = Project::all()
            .include(Project::fields().lead())
            .include(Project::fields().issues())
            .order_by(Project::fields().name().asc())
            .exec(&mut db(cx))
            .await?;
        Ok(projects)
    }

    /// Loads every project without its relations, sorted by name.
    pub async fn names(cx: &Cx) -> Result<Vec<Project>> {
        let projects = Project::all()
            .order_by(Project::fields().name().asc())
            .exec(&mut db(cx))
            .await?;
        Ok(projects)
    }

    /// Counts done and open issues. The issues must already be loaded.
    pub fn progress(&self) -> Progress {
        Progress::of(self.issues.get().iter().map(|issue| issue.status))
    }

    /// Validates the submitted fields and creates a project. Invalid fields return
    /// errors without creating anything.
    pub async fn start(cx: &Cx, form: &ProjectForm) -> Result<Result<Project, ProjectErrors>> {
        let valid = match form.validate(cx, None).await? {
            Ok(valid) => valid,
            Err(errors) => return Ok(Err(errors)),
        };
        let project = toasty::create!(Project {
            name: valid.name,
            code: valid.code,
            description: valid.description,
            lead_id: valid.lead_id,
            created_at: Timestamp::now(),
        })
        .exec(&mut db(cx))
        .await?;
        Ok(Ok(project))
    }

    /// Validates and saves the submitted fields. Invalid fields return errors
    /// without changing the project.
    pub async fn edit(&mut self, cx: &Cx, form: &ProjectForm) -> Result<Result<(), ProjectErrors>> {
        let valid = match form.validate(cx, Some(self.id)).await? {
            Ok(valid) => valid,
            Err(errors) => return Ok(Err(errors)),
        };
        self.update()
            .name(valid.name)
            .code(valid.code)
            .description(valid.description)
            .lead_id(valid.lead_id)
            .exec(&mut db(cx))
            .await?;
        Ok(Ok(()))
    }
}

/// How much of a set of issues is done.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub total: usize,
    pub done: usize,
}

impl Progress {
    pub fn of(statuses: impl Iterator<Item = IssueStatus>) -> Self {
        let mut progress = Self::default();
        for status in statuses {
            progress.total += 1;
            if status == IssueStatus::Done {
                progress.done += 1;
            }
        }
        progress
    }

    pub fn open(self) -> usize {
        self.total - self.done
    }

    /// Share of done issues in whole percent. An empty set is 0% done.
    pub fn percent(self) -> u8 {
        let percent = (self.done * 100).checked_div(self.total).unwrap_or(0);
        u8::try_from(percent).unwrap_or(100)
    }
}

/// Unvalidated form values. Keep them as strings so invalid input can be shown again.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProjectForm {
    pub name: String,
    pub code: String,
    pub description: String,
    /// The lead's user ID.
    pub lead: String,
}

struct ValidProject {
    name: String,
    code: String,
    description: String,
    lead_id: u64,
}

impl ProjectForm {
    pub fn of(project: &Project) -> Self {
        Self {
            name: project.name.clone(),
            code: project.code.clone(),
            description: project.description.clone(),
            lead: project.lead_id.to_string(),
        }
    }

    /// Checks every field. `own_id` is the project being edited, which may keep its key.
    async fn validate(
        &self,
        cx: &Cx,
        own_id: Option<u64>,
    ) -> Result<Result<ValidProject, ProjectErrors>> {
        let name = self.name();
        let mut code = self.code();
        let description = self.description();

        if let Ok(taken) = &code
            && let Some(other) = Project::filter_by_code(taken)
                .first()
                .exec(&mut db(cx))
                .await?
            && Some(other.id) != own_id
        {
            code = Err(ProjectError::CodeTaken);
        }

        let lead = match self.lead.trim().parse::<u64>() {
            Ok(id) if User::find(cx, id).await?.is_some() => Ok(id),
            _ => Err(ProjectError::LeadMissing),
        };

        Ok(match (name, code, description, lead) {
            (Ok(name), Ok(code), Ok(description), Ok(lead_id)) => Ok(ValidProject {
                name,
                code,
                description,
                lead_id,
            }),
            (name, code, description, lead) => Err(ProjectErrors {
                name: name.err(),
                code: code.err(),
                description: description.err(),
                lead: lead.err(),
            }),
        })
    }

    fn name(&self) -> Result<String, ProjectError> {
        let name = self.name.trim();
        if name.is_empty() {
            Err(ProjectError::NameMissing)
        } else if name.chars().count() > MAX_NAME {
            Err(ProjectError::NameTooLong { max: MAX_NAME })
        } else {
            Ok(name.to_owned())
        }
    }

    fn code(&self) -> Result<String, ProjectError> {
        let code = self.code.trim();
        if code.is_empty() {
            Err(ProjectError::CodeMissing)
        } else if !code.chars().all(|char| char.is_ascii_alphabetic())
            || !(MIN_CODE..=MAX_CODE).contains(&code.len())
        {
            Err(ProjectError::CodeInvalid {
                min: MIN_CODE,
                max: MAX_CODE,
            })
        } else {
            Ok(code.to_ascii_uppercase())
        }
    }

    fn description(&self) -> Result<String, ProjectError> {
        let description = self.description.trim();
        if description.chars().count() > MAX_DESCRIPTION {
            Err(ProjectError::DescriptionTooLong {
                max: MAX_DESCRIPTION,
            })
        } else {
            Ok(description.to_owned())
        }
    }
}

/// Why a [`ProjectForm`] field is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectError {
    NameMissing,
    /// The name has more than `max` characters.
    NameTooLong {
        max: usize,
    },
    CodeMissing,
    /// The key is not `min` to `max` letters.
    CodeInvalid {
        min: usize,
        max: usize,
    },
    /// Another project already uses the key.
    CodeTaken,
    /// The description has more than `max` characters.
    DescriptionTooLong {
        max: usize,
    },
    /// No lead was chosen, or the chosen user does not exist.
    LeadMissing,
}

/// Validation errors for [`ProjectForm`]. `None` means the field passed validation.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProjectErrors {
    pub name: Option<ProjectError>,
    pub code: Option<ProjectError>,
    pub description: Option<ProjectError>,
    pub lead: Option<ProjectError>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(name: &str, code: &str, description: &str) -> ProjectForm {
        ProjectForm {
            name: name.to_owned(),
            code: code.to_owned(),
            description: description.to_owned(),
            lead: String::new(),
        }
    }

    #[test]
    fn tidies_names_and_keys() {
        let form = form("  Web App ", " web ", " Storefront  ");
        assert_eq!(form.name().as_deref(), Ok("Web App"));
        assert_eq!(form.code().as_deref(), Ok("WEB"));
        assert_eq!(form.description().as_deref(), Ok("Storefront"));
    }

    #[test]
    fn rejects_invalid_fields() {
        let blank = form(" ", " ", "");
        assert_eq!(blank.name(), Err(ProjectError::NameMissing));
        assert_eq!(blank.code(), Err(ProjectError::CodeMissing));
        assert_eq!(blank.description().as_deref(), Ok(""));

        let long = form(
            &"a".repeat(MAX_NAME + 1),
            "TOOLONG",
            &"a".repeat(MAX_DESCRIPTION + 1),
        );
        assert!(long.name().is_err());
        assert!(long.code().is_err());
        assert!(long.description().is_err());

        for code in ["W", "W3B", "WE B", "WE-B"] {
            assert!(form("Web", code, "").code().is_err(), "{code:?}");
        }
    }

    #[test]
    fn progress_counts_done_issues() {
        let progress = Progress::of(
            [
                IssueStatus::Done,
                IssueStatus::Todo,
                IssueStatus::Done,
                IssueStatus::InProgress,
            ]
            .into_iter(),
        );
        assert_eq!((progress.total, progress.done, progress.open()), (4, 2, 2));
        assert_eq!(progress.percent(), 50);
        assert_eq!(Progress::default().percent(), 0);
    }
}
