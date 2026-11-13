use std::cmp::Reverse;

use jiff::civil::Date;
use serde::Deserialize;
use topcoat::{Result, context::Cx};

use super::issue::Issue;
use crate::db::db;

/// Maximum name length in characters.
const MAX_NAME: usize = 60;

/// Maximum goal length in characters.
const MAX_GOAL: usize = 280;

/// A time-boxed set of a project's issues.
#[derive(Debug, toasty::Model)]
pub struct Sprint {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub project_id: u64,

    pub name: String,

    /// What the team wants to achieve. May be empty.
    pub goal: String,

    pub starts_on: Date,

    /// The last day of the sprint.
    pub ends_on: Date,

    pub state: SprintState,
}

impl Sprint {
    pub async fn find(cx: &Cx, id: u64) -> Result<Option<Sprint>> {
        let sprint = Sprint::filter_by_id(id).first().exec(&mut db(cx)).await?;
        Ok(sprint)
    }

    /// Loads a project's sprints: the active one first, then planned ones by start
    /// date, then completed ones with the most recent first.
    pub async fn of_project(cx: &Cx, project_id: u64) -> Result<Vec<Sprint>> {
        let mut sprints = Sprint::filter_by_project_id(project_id)
            .exec(&mut db(cx))
            .await?;
        sprints.sort_by_key(|sprint| match sprint.state {
            SprintState::Active => (0, Ok(sprint.starts_on)),
            SprintState::Planned => (1, Ok(sprint.starts_on)),
            SprintState::Completed => (2, Err(Reverse(sprint.ends_on))),
        });
        Ok(sprints)
    }

    /// Whether issues can still be added to the sprint.
    pub fn is_open(&self) -> bool {
        self.state != SprintState::Completed
    }

    /// Validates the submitted fields and plans a sprint for the project. Invalid
    /// fields return errors without adding anything.
    pub async fn plan(
        cx: &Cx,
        project_id: u64,
        form: &SprintForm,
    ) -> Result<Result<Sprint, SprintErrors>> {
        let valid = match form.validate() {
            Ok(valid) => valid,
            Err(errors) => return Ok(Err(errors)),
        };
        let sprint = toasty::create!(Sprint {
            project_id,
            name: valid.name,
            goal: valid.goal,
            starts_on: valid.starts_on,
            ends_on: valid.ends_on,
            state: SprintState::Planned,
        })
        .exec(&mut db(cx))
        .await?;
        Ok(Ok(sprint))
    }

    /// Starts a planned sprint. A project runs one sprint at a time, so this returns
    /// `false` without changing anything while another sprint is active.
    pub async fn start(&mut self, cx: &Cx) -> Result<bool> {
        if self.state != SprintState::Planned {
            return Ok(false);
        }
        let mut db = db(cx);
        let fields = Sprint::fields();
        let active = Sprint::filter(
            fields
                .project_id()
                .eq(self.project_id)
                .and(fields.state().eq(SprintState::Active)),
        )
        .first()
        .exec(&mut db)
        .await?;
        if active.is_some() {
            return Ok(false);
        }
        self.update()
            .state(SprintState::Active)
            .exec(&mut db)
            .await?;
        Ok(true)
    }

    /// Completes the active sprint. Issues that are not done go back to the backlog.
    /// Returns `false` without changing anything if the sprint is not active.
    pub async fn complete(&mut self, cx: &Cx) -> Result<bool> {
        if self.state != SprintState::Active {
            return Ok(false);
        }
        Issue::leave_sprint(cx, self.id, false).await?;
        self.update()
            .state(SprintState::Completed)
            .exec(&mut db(cx))
            .await?;
        Ok(true)
    }

    /// Deletes the sprint. Its issues go back to the backlog.
    pub async fn remove(self, cx: &Cx) -> Result<()> {
        Issue::leave_sprint(cx, self.id, true).await?;
        Sprint::filter_by_id(self.id)
            .delete()
            .exec(&mut db(cx))
            .await?;
        Ok(())
    }

    /// Deletes every sprint of a project without touching its issues.
    pub async fn remove_all(cx: &Cx, project_id: u64) -> Result<()> {
        Sprint::filter_by_project_id(project_id)
            .delete()
            .exec(&mut db(cx))
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum SprintState {
    /// Not started yet.
    Planned,
    Active,
    Completed,
}

/// Unvalidated form values. Keep them as strings so invalid input can be shown again.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SprintForm {
    pub name: String,
    pub goal: String,
    /// The first day as `YYYY-MM-DD`.
    pub starts_on: String,
    /// The last day as `YYYY-MM-DD`.
    pub ends_on: String,
}

struct ValidSprint {
    name: String,
    goal: String,
    starts_on: Date,
    ends_on: Date,
}

impl SprintForm {
    fn validate(&self) -> Result<ValidSprint, SprintErrors> {
        let name = self.name();
        let goal = self.goal();
        let starts_on = date(&self.starts_on);
        let mut ends_on = date(&self.ends_on);
        if let (Ok(start), Ok(end)) = (starts_on, ends_on)
            && end < start
        {
            ends_on = Err(SprintError::EndBeforeStart);
        }

        match (name, goal, starts_on, ends_on) {
            (Ok(name), Ok(goal), Ok(starts_on), Ok(ends_on)) => Ok(ValidSprint {
                name,
                goal,
                starts_on,
                ends_on,
            }),
            (name, goal, starts_on, ends_on) => Err(SprintErrors {
                name: name.err(),
                goal: goal.err(),
                starts_on: starts_on.err(),
                ends_on: ends_on.err(),
            }),
        }
    }

    fn name(&self) -> Result<String, SprintError> {
        let name = self.name.trim();
        if name.is_empty() {
            Err(SprintError::NameMissing)
        } else if name.chars().count() > MAX_NAME {
            Err(SprintError::NameTooLong { max: MAX_NAME })
        } else {
            Ok(name.to_owned())
        }
    }

    fn goal(&self) -> Result<String, SprintError> {
        let goal = self.goal.trim();
        if goal.chars().count() > MAX_GOAL {
            Err(SprintError::GoalTooLong { max: MAX_GOAL })
        } else {
            Ok(goal.to_owned())
        }
    }
}

fn date(text: &str) -> Result<Date, SprintError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(SprintError::DateMissing);
    }
    text.parse().map_err(|_| SprintError::DateInvalid)
}

/// Why a [`SprintForm`] field is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprintError {
    NameMissing,
    /// The name has more than `max` characters.
    NameTooLong {
        max: usize,
    },
    /// The goal has more than `max` characters.
    GoalTooLong {
        max: usize,
    },
    DateMissing,
    /// The text is not a calendar date.
    DateInvalid,
    /// The last day is before the first day.
    EndBeforeStart,
}

/// Validation errors for [`SprintForm`]. `None` means the field passed validation.
#[derive(Debug, Clone, Copy, Default)]
pub struct SprintErrors {
    pub name: Option<SprintError>,
    pub goal: Option<SprintError>,
    pub starts_on: Option<SprintError>,
    pub ends_on: Option<SprintError>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(name: &str, starts_on: &str, ends_on: &str) -> SprintForm {
        SprintForm {
            name: name.to_owned(),
            goal: String::new(),
            starts_on: starts_on.to_owned(),
            ends_on: ends_on.to_owned(),
        }
    }

    #[test]
    fn accepts_a_sprint_of_one_day() {
        let valid = form(" Sprint 4 ", "2026-11-16", "2026-11-16").validate();
        let valid = valid.expect("the sprint is valid");
        assert_eq!(valid.name, "Sprint 4");
        assert_eq!(valid.starts_on, valid.ends_on);
    }

    #[test]
    fn rejects_invalid_fields() {
        let errors = form(" ", "", "soon").validate().err().unwrap();
        assert_eq!(errors.name, Some(SprintError::NameMissing));
        assert_eq!(errors.starts_on, Some(SprintError::DateMissing));
        assert_eq!(errors.ends_on, Some(SprintError::DateInvalid));

        let errors = form("Sprint 4", "2026-11-16", "2026-11-15")
            .validate()
            .err()
            .unwrap();
        assert_eq!(errors.starts_on, None);
        assert_eq!(errors.ends_on, Some(SprintError::EndBeforeStart));
    }
}
