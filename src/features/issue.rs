use std::str::FromStr;

use jiff::{Timestamp, civil::Date};
use serde::{Deserialize, Serialize};
use toasty::{
    schema::Model,
    stmt::{Expr, List},
};
use toasty_core::driver::operation::TransactionMode;
use tokio::sync::Mutex;
use topcoat::{Result, context::Cx};

use super::{project::Project, user::User};
use crate::db::{contains_pattern, db};

/// Maximum title length in characters.
const MAX_TITLE: usize = 120;

/// Maximum description length in characters.
const MAX_DESCRIPTION: usize = 5_000;

/// Maximum comment length in characters.
const MAX_COMMENT: usize = 2_000;

/// Held while an issue is given its number. See [`Issue::open`].
static NUMBERING: Mutex<()> = Mutex::const_new(());

#[derive(Debug, toasty::Model)]
#[index(status)]
// The database rejects a second issue with the same number in a project.
#[unique(project_id, number)]
pub struct Issue {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub project_id: u64,

    #[belongs_to]
    pub project: toasty::Deferred<Project>,

    /// Position within the project, starting at 1. Shown in the issue key, such as
    /// the `12` in `WEB-12`.
    pub number: u32,

    pub title: String,

    pub description: String,

    pub kind: IssueKind,

    pub status: IssueStatus,

    pub priority: Priority,

    #[index]
    pub assignee_id: Option<u64>,

    #[belongs_to]
    pub assignee: toasty::Deferred<Option<User>>,

    #[index]
    pub reporter_id: u64,

    #[belongs_to]
    pub reporter: toasty::Deferred<User>,

    #[has_many]
    pub comments: toasty::Deferred<Vec<Comment>>,

    pub due_on: Option<Date>,

    pub created_at: Timestamp,

    /// Changes whenever the issue is edited, moved, or commented on.
    pub updated_at: Timestamp,

    /// Set while the status is [`IssueStatus::Done`].
    pub completed_at: Option<Timestamp>,
}

impl Issue {
    /// Loads an issue with its project, assignee, and reporter.
    pub async fn find(cx: &Cx, id: u64) -> Result<Option<Issue>> {
        let issue = Issue::filter_by_id(id)
            .include(Issue::fields().project())
            .include(Issue::fields().assignee())
            .include(Issue::fields().reporter())
            .first()
            .exec(&mut db(cx))
            .await?;
        Ok(issue)
    }

    /// Loads a project's issues and their assignees. Each board column wants the most
    /// urgent issues first, then the most recently updated.
    pub async fn of_project(cx: &Cx, project_id: u64) -> Result<Vec<Issue>> {
        let mut issues = Issue::filter_by_project_id(project_id)
            .include(Issue::fields().assignee())
            .exec(&mut db(cx))
            .await?;
        issues.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(b.updated_at.cmp(&a.updated_at))
        });
        Ok(issues)
    }

    /// Loads the issues assigned to a user with their projects, most urgent first.
    /// Skips done issues unless `include_done` is set.
    pub async fn assigned_to(cx: &Cx, user_id: u64, include_done: bool) -> Result<Vec<Issue>> {
        let mut filter = Issue::fields().assignee_id().eq(user_id);
        if !include_done {
            filter = filter.and(Issue::fields().status().ne(IssueStatus::Done));
        }
        let mut issues = Issue::filter(filter)
            .include(Issue::fields().project())
            .exec(&mut db(cx))
            .await?;
        issues.sort_by(|a, b| {
            (a.status == IssueStatus::Done)
                .cmp(&(b.status == IssueStatus::Done))
                .then(b.priority.cmp(&a.priority))
                .then(b.updated_at.cmp(&a.updated_at))
        });
        Ok(issues)
    }

    /// Loads every issue without its relations.
    pub async fn all_plain(cx: &Cx) -> Result<Vec<Issue>> {
        let issues = Issue::all().exec(&mut db(cx)).await?;
        Ok(issues)
    }

    /// The issue key, such as `WEB-12`. The project must already be loaded.
    pub fn key(&self) -> String {
        format!("{}-{}", self.project.get().code, self.number)
    }

    /// Whether the due date has passed and the issue is not done.
    pub fn is_overdue(&self, today: Date) -> bool {
        self.status != IssueStatus::Done && self.due_on.is_some_and(|due| due < today)
    }

    /// Validates the submitted fields and creates an issue reported by `reporter`.
    /// Invalid fields return errors without creating anything.
    pub async fn open(
        cx: &Cx,
        reporter: &User,
        form: &IssueForm,
    ) -> Result<Result<Issue, IssueErrors>> {
        let project = match form.project.trim().parse::<u64>() {
            Ok(id) => Project::find(cx, id).await?,
            Err(_) => None,
        };
        let valid = form.validate(cx).await?;
        let (project, valid) = match (project, valid) {
            (Some(project), Ok(valid)) => (project, valid),
            (project, valid) => {
                let mut errors = valid.err().unwrap_or_default();
                if project.is_none() {
                    errors.project = Some(IssueError::ProjectMissing);
                }
                return Ok(Err(errors));
            }
        };

        // Number one issue at a time. Requests in this process queue here without
        // holding a database connection. The transaction then takes the database's
        // write lock before reading the last number, which also keeps out other
        // processes that share the database file.
        let _numbering = NUMBERING.lock().await;
        let mut db = db(cx);
        let mut tx = db
            .transaction_builder()
            .mode(TransactionMode::Immediate)
            .begin()
            .await?;
        let last = Issue::filter_by_project_id(project.id)
            .order_by(Issue::fields().number().desc())
            .first()
            .exec(&mut tx)
            .await?;
        let now = Timestamp::now();

        let issue = toasty::create!(Issue {
            project_id: project.id,
            number: last.map_or(1, |issue| issue.number + 1),
            title: valid.title,
            description: valid.description,
            kind: form.kind,
            status: form.status,
            priority: form.priority,
            assignee_id: valid.assignee_id,
            reporter_id: reporter.id,
            due_on: valid.due_on,
            created_at: now,
            updated_at: now,
            completed_at: (form.status == IssueStatus::Done).then_some(now),
        })
        .exec(&mut tx)
        .await?;
        tx.commit().await?;
        Ok(Ok(issue))
    }

    /// Validates and saves the submitted fields. Invalid fields return errors
    /// without changing the issue. The project cannot change.
    pub async fn edit(&mut self, cx: &Cx, form: &IssueForm) -> Result<Result<(), IssueErrors>> {
        let valid = match form.validate(cx).await? {
            Ok(valid) => valid,
            Err(errors) => return Ok(Err(errors)),
        };
        let now = Timestamp::now();
        let completed_at = self.completed_after(form.status, now);
        self.update()
            .title(valid.title)
            .description(valid.description)
            .kind(form.kind)
            .status(form.status)
            .priority(form.priority)
            .assignee_id(valid.assignee_id)
            .due_on(valid.due_on)
            .updated_at(now)
            .completed_at(completed_at)
            .exec(&mut db(cx))
            .await?;
        Ok(Ok(()))
    }

    /// Moves the issue to `status`, such as when its card is dropped in a board column.
    pub async fn move_to(&mut self, cx: &Cx, status: IssueStatus) -> Result<()> {
        if self.status == status {
            return Ok(());
        }
        let now = Timestamp::now();
        let completed_at = self.completed_after(status, now);
        self.update()
            .status(status)
            .updated_at(now)
            .completed_at(completed_at)
            .exec(&mut db(cx))
            .await?;
        Ok(())
    }

    /// Deletes the issue and its comments.
    pub async fn remove(self, cx: &Cx) -> Result<()> {
        let mut db = db(cx);
        Comment::filter_by_issue_id(self.id)
            .delete()
            .exec(&mut db)
            .await?;
        Issue::filter_by_id(self.id).delete().exec(&mut db).await?;
        Ok(())
    }

    /// The completion time after moving to `status` at `now`. An issue that stays
    /// done keeps its original completion time.
    fn completed_after(&self, status: IssueStatus, now: Timestamp) -> Option<Timestamp> {
        match status {
            IssueStatus::Done => self.completed_at.or(Some(now)),
            _ => None,
        }
    }
}

/// A board column. Issues may move between any two statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, toasty::Embed)]
#[serde(rename_all = "kebab-case")]
pub enum IssueStatus {
    /// Not planned yet.
    Backlog,
    /// Planned and ready to start.
    Todo,
    InProgress,
    InReview,
    Done,
}

impl IssueStatus {
    /// The statuses in board order.
    pub const ALL: [Self; 5] = [
        Self::Backlog,
        Self::Todo,
        Self::InProgress,
        Self::InReview,
        Self::Done,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Todo => "todo",
            Self::InProgress => "in-progress",
            Self::InReview => "in-review",
            Self::Done => "done",
        }
    }

    /// The column to the left on the board.
    pub fn previous(self) -> Option<Self> {
        let index = Self::ALL.iter().position(|&status| status == self)?;
        index.checked_sub(1).map(|index| Self::ALL[index])
    }

    /// The column to the right on the board.
    pub fn next(self) -> Option<Self> {
        let index = Self::ALL.iter().position(|&status| status == self)?;
        Self::ALL.get(index + 1).copied()
    }
}

impl FromStr for IssueStatus {
    type Err = UnknownChoice;

    fn from_str(slug: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.slug() == slug)
            .ok_or(UnknownChoice)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, toasty::Embed)]
#[serde(rename_all = "kebab-case")]
pub enum IssueKind {
    Task,
    Bug,
    Feature,
}

impl IssueKind {
    pub const ALL: [Self; 3] = [Self::Task, Self::Bug, Self::Feature];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Bug => "bug",
            Self::Feature => "feature",
        }
    }
}

/// How urgent an issue is, from least to most urgent.
///
/// The stored labels start with a digit so the database sorts them by urgency.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, toasty::Embed,
)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    #[column(variant = "1_low")]
    Low,
    #[column(variant = "2_medium")]
    Medium,
    #[column(variant = "3_high")]
    High,
    #[column(variant = "4_urgent")]
    Urgent,
}

impl Priority {
    /// The priorities from most to least urgent.
    pub const ALL: [Self; 4] = [Self::Urgent, Self::High, Self::Medium, Self::Low];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

impl FromStr for Priority {
    type Err = UnknownChoice;

    fn from_str(slug: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|priority| priority.slug() == slug)
            .ok_or(UnknownChoice)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueSort {
    #[default]
    Updated,
    Newest,
    Oldest,
    Priority,
}

impl IssueSort {
    pub const ALL: [Self; 4] = [Self::Updated, Self::Newest, Self::Oldest, Self::Priority];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Updated => "updated",
            Self::Newest => "newest",
            Self::Oldest => "oldest",
            Self::Priority => "priority",
        }
    }
}

impl FromStr for IssueSort {
    type Err = UnknownChoice;

    fn from_str(slug: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|sort| sort.slug() == slug)
            .ok_or(UnknownChoice)
    }
}

/// A slug that names no status, priority, or sort order.
#[derive(Debug, PartialEq, Eq)]
pub struct UnknownChoice;

#[derive(Debug, Default)]
pub struct IssueSearch<'a> {
    pub project: Option<u64>,
    pub status: Option<IssueStatus>,
    pub priority: Option<Priority>,
    /// Searches titles, project keys, and issue keys such as `WEB-12`.
    /// Blank text matches all issues.
    pub text: &'a str,
}

impl IssueSearch<'_> {
    /// Loads one page of matching issues with their projects and assignees.
    /// Page numbers start at 1.
    pub async fn page(
        &self,
        cx: &Cx,
        sort: IssueSort,
        page: usize,
        per_page: usize,
    ) -> Result<Vec<Issue>> {
        let fields = Issue::fields();
        let issues = self
            .query()
            .include(Issue::fields().project())
            .include(Issue::fields().assignee())
            .order_by(match sort {
                IssueSort::Updated => fields.updated_at().desc(),
                // Issue IDs grow in creation order.
                IssueSort::Newest => fields.id().desc(),
                IssueSort::Oldest => fields.id().asc(),
                // The stored labels sort by urgency.
                IssueSort::Priority => fields.priority().path.desc(),
            })
            .limit(per_page)
            .offset(page.saturating_sub(1) * per_page)
            .exec(&mut db(cx))
            .await?;
        Ok(issues)
    }

    pub async fn count(&self, cx: &Cx) -> Result<usize> {
        let count = self.query().count().exec(&mut db(cx)).await?;
        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    fn query(&self) -> <Issue as Model>::Query<List<Issue>> {
        let fields = Issue::fields();
        let project = self.project.map(|id| fields.project_id().eq(id));
        let status = self.status.map(|status| fields.status().eq(status));
        let priority = self.priority.map(|priority| fields.priority().eq(priority));

        let text = self.text.trim();
        let text = (!text.is_empty()).then(|| {
            let pattern = contains_pattern(text);
            let mut matches = fields
                .title()
                .like_with_escape(pattern.clone(), '\\')
                .or(fields.project().code().like_with_escape(pattern, '\\'));
            if let Some((code, number)) = parse_key(text) {
                let number = fields.number().eq(number);
                matches = matches.or(match code {
                    Some(code) => fields.project().code().eq(code).and(number),
                    None => number,
                });
            }
            matches
        });

        match [project, status, priority, text]
            .into_iter()
            .flatten()
            .reduce(Expr::and)
        {
            Some(filter) => Issue::filter(filter),
            None => Issue::all(),
        }
    }
}

/// Reads an issue key such as `web-12` or a bare number such as `#12`.
/// Returns the uppercase project key, if any, and the number.
fn parse_key(text: &str) -> Option<(Option<String>, u32)> {
    match text.rsplit_once('-') {
        Some((code, number)) if !code.is_empty() => {
            Some((Some(code.trim().to_ascii_uppercase()), number.parse().ok()?))
        }
        Some(_) => None,
        None => Some((None, text.trim_start_matches('#').parse().ok()?)),
    }
}

/// Form values for creating or editing an issue. Free text stays as strings so
/// invalid input can be shown again. A choice outside its list fails to parse and
/// rejects the whole request.
#[derive(Debug, Clone, Deserialize)]
pub struct IssueForm {
    /// The project's ID. Only used when creating an issue.
    #[serde(default)]
    pub project: String,
    pub title: String,
    pub description: String,
    pub kind: IssueKind,
    pub status: IssueStatus,
    pub priority: Priority,
    /// The assignee's user ID. Blank leaves the issue unassigned.
    pub assignee: String,
    /// The due date as `YYYY-MM-DD`. Blank means no due date.
    pub due_on: String,
}

struct ValidIssue {
    title: String,
    description: String,
    assignee_id: Option<u64>,
    due_on: Option<Date>,
}

impl IssueForm {
    /// An empty form for a new issue, with the project preselected if given.
    pub fn new(project: Option<u64>) -> Self {
        Self {
            project: project.map(|id| id.to_string()).unwrap_or_default(),
            title: String::new(),
            description: String::new(),
            kind: IssueKind::Task,
            status: IssueStatus::Todo,
            priority: Priority::Medium,
            assignee: String::new(),
            due_on: String::new(),
        }
    }

    pub fn of(issue: &Issue) -> Self {
        Self {
            project: issue.project_id.to_string(),
            title: issue.title.clone(),
            description: issue.description.clone(),
            kind: issue.kind,
            status: issue.status,
            priority: issue.priority,
            assignee: issue
                .assignee_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            due_on: issue.due_on.map(|due| due.to_string()).unwrap_or_default(),
        }
    }

    /// Checks every field except the project.
    async fn validate(&self, cx: &Cx) -> Result<Result<ValidIssue, IssueErrors>> {
        let title = self.title();
        let description = self.description();
        let due_on = self.due_on();

        let assignee = self.assignee.trim();
        let assignee = if assignee.is_empty() {
            Ok(None)
        } else {
            match assignee.parse::<u64>() {
                Ok(id) if User::find(cx, id).await?.is_some() => Ok(Some(id)),
                _ => Err(IssueError::AssigneeUnknown),
            }
        };

        Ok(match (title, description, assignee, due_on) {
            (Ok(title), Ok(description), Ok(assignee_id), Ok(due_on)) => Ok(ValidIssue {
                title,
                description,
                assignee_id,
                due_on,
            }),
            (title, description, assignee, due_on) => Err(IssueErrors {
                project: None,
                title: title.err(),
                description: description.err(),
                assignee: assignee.err(),
                due_on: due_on.err(),
            }),
        })
    }

    fn title(&self) -> Result<String, IssueError> {
        let title = self.title.trim();
        if title.is_empty() {
            Err(IssueError::TitleMissing)
        } else if title.chars().count() > MAX_TITLE {
            Err(IssueError::TitleTooLong { max: MAX_TITLE })
        } else {
            Ok(title.to_owned())
        }
    }

    fn description(&self) -> Result<String, IssueError> {
        // Browsers submit line breaks as CRLF.
        let description = self.description.trim().replace("\r\n", "\n");
        if description.chars().count() > MAX_DESCRIPTION {
            Err(IssueError::DescriptionTooLong {
                max: MAX_DESCRIPTION,
            })
        } else {
            Ok(description)
        }
    }

    fn due_on(&self) -> Result<Option<Date>, IssueError> {
        let due_on = self.due_on.trim();
        if due_on.is_empty() {
            return Ok(None);
        }
        due_on
            .parse()
            .map(Some)
            .map_err(|_| IssueError::DueDateInvalid)
    }
}

/// Why an [`IssueForm`] field is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueError {
    /// No project was chosen, or the chosen project does not exist.
    ProjectMissing,
    TitleMissing,
    /// The title has more than `max` characters.
    TitleTooLong {
        max: usize,
    },
    /// The description has more than `max` characters.
    DescriptionTooLong {
        max: usize,
    },
    /// The chosen assignee does not exist.
    AssigneeUnknown,
    /// The due date is not a calendar date.
    DueDateInvalid,
}

/// Validation errors for [`IssueForm`]. `None` means the field passed validation.
#[derive(Debug, Clone, Copy, Default)]
pub struct IssueErrors {
    pub project: Option<IssueError>,
    pub title: Option<IssueError>,
    pub description: Option<IssueError>,
    pub assignee: Option<IssueError>,
    pub due_on: Option<IssueError>,
}

#[derive(Debug, toasty::Model)]
pub struct Comment {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub issue_id: u64,

    #[belongs_to]
    pub issue: toasty::Deferred<Issue>,

    #[index]
    pub author_id: u64,

    #[belongs_to]
    pub author: toasty::Deferred<User>,

    pub body: String,

    pub created_at: Timestamp,
}

impl Comment {
    /// Loads an issue's comments and their authors, oldest first.
    pub async fn of_issue(cx: &Cx, issue_id: u64) -> Result<Vec<Comment>> {
        let comments = Comment::filter_by_issue_id(issue_id)
            .include(Comment::fields().author())
            .order_by(Comment::fields().id().asc())
            .exec(&mut db(cx))
            .await?;
        Ok(comments)
    }

    /// Adds a comment to `issue` and marks the issue as updated. An invalid body
    /// returns an error without adding anything.
    pub async fn add(
        cx: &Cx,
        issue: &mut Issue,
        author: &User,
        body: &str,
    ) -> Result<Result<(), CommentError>> {
        let body = match comment_body(body) {
            Ok(body) => body,
            Err(error) => return Ok(Err(error)),
        };
        let mut db = db(cx);
        let now = Timestamp::now();
        toasty::create!(Comment {
            issue_id: issue.id,
            author_id: author.id,
            body,
            created_at: now,
        })
        .exec(&mut db)
        .await?;
        issue.update().updated_at(now).exec(&mut db).await?;
        Ok(Ok(()))
    }
}

fn comment_body(body: &str) -> Result<String, CommentError> {
    let body = body.trim().replace("\r\n", "\n");
    if body.is_empty() {
        Err(CommentError::Missing)
    } else if body.chars().count() > MAX_COMMENT {
        Err(CommentError::TooLong { max: MAX_COMMENT })
    } else {
        Ok(body)
    }
}

/// Why a comment was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentError {
    Missing,
    /// The comment has more than `max` characters.
    TooLong {
        max: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(title: &str, description: &str, due_on: &str) -> IssueForm {
        IssueForm {
            title: title.to_owned(),
            description: description.to_owned(),
            due_on: due_on.to_owned(),
            ..IssueForm::new(None)
        }
    }

    #[test]
    fn statuses_step_through_the_board() {
        assert_eq!(IssueStatus::Backlog.previous(), None);
        assert_eq!(IssueStatus::Backlog.next(), Some(IssueStatus::Todo));
        assert_eq!(IssueStatus::InReview.next(), Some(IssueStatus::Done));
        assert_eq!(IssueStatus::Done.previous(), Some(IssueStatus::InReview));
        assert_eq!(IssueStatus::Done.next(), None);
    }

    #[test]
    fn slugs_read_back() {
        for status in IssueStatus::ALL {
            assert_eq!(status.slug().parse(), Ok(status));
        }
        for priority in Priority::ALL {
            assert_eq!(priority.slug().parse(), Ok(priority));
        }
        for sort in IssueSort::ALL {
            assert_eq!(sort.slug().parse(), Ok(sort));
        }
    }

    #[test]
    fn urgent_outranks_low() {
        assert!(Priority::Urgent > Priority::High);
        assert!(Priority::Medium > Priority::Low);
    }

    #[test]
    fn reads_issue_keys() {
        assert_eq!(parse_key("web-12"), Some((Some("WEB".to_owned()), 12)));
        assert_eq!(parse_key("#12"), Some((None, 12)));
        assert_eq!(parse_key("12"), Some((None, 12)));
        assert_eq!(parse_key("sign-in"), None);
        assert_eq!(parse_key("-12"), None);
        assert_eq!(parse_key("checkout"), None);
    }

    #[test]
    fn tidies_text_fields() {
        let form = form("  Fix checkout ", " Steps:\r\n1. Pay ", " ");
        assert_eq!(form.title().as_deref(), Ok("Fix checkout"));
        assert_eq!(form.description().as_deref(), Ok("Steps:\n1. Pay"));
        assert_eq!(form.due_on(), Ok(None));
    }

    #[test]
    fn rejects_invalid_fields() {
        assert_eq!(form(" ", "", "").title(), Err(IssueError::TitleMissing));
        assert!(form(&"a".repeat(MAX_TITLE + 1), "", "").title().is_err());
        assert!(
            form("Fix", &"a".repeat(MAX_DESCRIPTION + 1), "")
                .description()
                .is_err()
        );
        assert_eq!(
            form("Fix", "", "next week").due_on(),
            Err(IssueError::DueDateInvalid)
        );
        assert_eq!(
            form("Fix", "", "2026-11-30").due_on(),
            Ok(Some(Date::constant(2026, 11, 30)))
        );
    }

    #[test]
    fn rejects_blank_and_long_comments() {
        assert_eq!(comment_body(" \r\n "), Err(CommentError::Missing));
        assert!(comment_body(&"a".repeat(MAX_COMMENT + 1)).is_err());
        assert_eq!(comment_body(" Looks good. ").as_deref(), Ok("Looks good."));
    }
}
