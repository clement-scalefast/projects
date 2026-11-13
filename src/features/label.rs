use std::collections::HashMap;

use serde::Deserialize;
use topcoat::{Result, context::Cx};

use super::issue::Issue;
use crate::db::db;

/// Maximum name length in characters.
const MAX_NAME: usize = 30;

/// A tag that groups issues across projects, such as `performance`.
#[derive(Debug, toasty::Model)]
pub struct Label {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub name: String,

    pub color: LabelColor,
}

impl Label {
    pub async fn find(cx: &Cx, id: u64) -> Result<Option<Label>> {
        let label = Label::filter_by_id(id).first().exec(&mut db(cx)).await?;
        Ok(label)
    }

    /// Loads every label, sorted by name.
    pub async fn list(cx: &Cx) -> Result<Vec<Label>> {
        let labels = Label::all()
            .order_by(Label::fields().name().asc())
            .exec(&mut db(cx))
            .await?;
        Ok(labels)
    }

    /// Validates the submitted fields and adds a label. An invalid name returns an
    /// error without adding anything.
    pub async fn add(cx: &Cx, form: &LabelForm) -> Result<Result<Label, LabelError>> {
        let name = match form.name() {
            Ok(name) => name,
            Err(error) => return Ok(Err(error)),
        };
        let mut db = db(cx);
        if Label::filter_by_name(&name)
            .first()
            .exec(&mut db)
            .await?
            .is_some()
        {
            return Ok(Err(LabelError::Taken));
        }
        let label = toasty::create!(Label {
            name,
            color: form.color,
        })
        .exec(&mut db)
        .await?;
        Ok(Ok(label))
    }

    /// Deletes the label and takes it off every issue.
    pub async fn remove(self, cx: &Cx) -> Result<()> {
        let mut db = db(cx);
        IssueLabel::filter_by_label_id(self.id)
            .delete()
            .exec(&mut db)
            .await?;
        Label::filter_by_id(self.id).delete().exec(&mut db).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, toasty::Embed)]
#[serde(rename_all = "lowercase")]
pub enum LabelColor {
    Gray,
    Red,
    Amber,
    Green,
    Sky,
    Violet,
    Pink,
}

impl LabelColor {
    pub const ALL: [Self; 7] = [
        Self::Gray,
        Self::Red,
        Self::Amber,
        Self::Green,
        Self::Sky,
        Self::Violet,
        Self::Pink,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Gray => "gray",
            Self::Red => "red",
            Self::Amber => "amber",
            Self::Green => "green",
            Self::Sky => "sky",
            Self::Violet => "violet",
            Self::Pink => "pink",
        }
    }
}

/// Puts a label on an issue.
#[derive(Debug, toasty::Model)]
#[unique(issue_id, label_id)]
pub struct IssueLabel {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub issue_id: u64,

    #[index]
    pub label_id: u64,
}

impl IssueLabel {
    /// Puts the label on the issue, or takes it off if the issue already has it.
    pub async fn toggle(cx: &Cx, issue_id: u64, label_id: u64) -> Result<()> {
        let mut db = db(cx);
        let fields = IssueLabel::fields();
        let pair = fields
            .issue_id()
            .eq(issue_id)
            .and(fields.label_id().eq(label_id));
        if IssueLabel::filter(pair.clone())
            .first()
            .exec(&mut db)
            .await?
            .is_some()
        {
            IssueLabel::filter(pair).delete().exec(&mut db).await?;
        } else {
            toasty::create!(IssueLabel { issue_id, label_id })
                .exec(&mut db)
                .await?;
        }
        Ok(())
    }

    /// Takes every label off an issue.
    pub async fn clear(cx: &Cx, issue_id: u64) -> Result<()> {
        IssueLabel::filter_by_issue_id(issue_id)
            .delete()
            .exec(&mut db(cx))
            .await?;
        Ok(())
    }

    /// Counts the issues of each label, by label ID.
    pub async fn usage(cx: &Cx) -> Result<HashMap<u64, usize>> {
        let links = IssueLabel::all().exec(&mut db(cx)).await?;
        let mut usage = HashMap::new();
        for link in links {
            *usage.entry(link.label_id).or_default() += 1;
        }
        Ok(usage)
    }

    /// Returns the IDs of the issues that have a label.
    pub async fn issues_with(cx: &Cx, label_id: u64) -> Result<Vec<u64>> {
        let links = IssueLabel::filter_by_label_id(label_id)
            .exec(&mut db(cx))
            .await?;
        Ok(links.into_iter().map(|link| link.issue_id).collect())
    }
}

/// The labels of a set of issues, loaded with two queries.
#[derive(Debug, Default)]
pub struct IssueLabels {
    /// Every label, sorted by name.
    pub all: Vec<Label>,
    /// Label IDs by issue ID.
    by_issue: HashMap<u64, Vec<u64>>,
}

impl IssueLabels {
    pub async fn load(cx: &Cx, issues: &[Issue]) -> Result<Self> {
        let all = Label::list(cx).await?;
        let mut by_issue = HashMap::<u64, Vec<u64>>::new();
        if !all.is_empty() && !issues.is_empty() {
            let ids = issues.iter().map(|issue| issue.id).collect::<Vec<_>>();
            let links = IssueLabel::filter(IssueLabel::fields().issue_id().in_list(ids))
                .exec(&mut db(cx))
                .await?;
            for link in links {
                by_issue
                    .entry(link.issue_id)
                    .or_default()
                    .push(link.label_id);
            }
        }
        Ok(Self { all, by_issue })
    }

    /// The labels of an issue, sorted by name.
    pub fn of(&self, issue_id: u64) -> impl Iterator<Item = &Label> {
        let ids = self.by_issue.get(&issue_id);
        self.all
            .iter()
            .filter(move |label| ids.is_some_and(|ids| ids.contains(&label.id)))
    }
}

/// Form values for adding a label. A color outside the list fails to parse and
/// rejects the whole request.
#[derive(Debug, Clone, Deserialize)]
pub struct LabelForm {
    pub name: String,
    pub color: LabelColor,
}

impl LabelForm {
    fn name(&self) -> Result<String, LabelError> {
        let name = self.name.trim();
        if name.is_empty() {
            Err(LabelError::Missing)
        } else if name.chars().count() > MAX_NAME {
            Err(LabelError::TooLong { max: MAX_NAME })
        } else {
            Ok(name.to_owned())
        }
    }
}

/// Why a label was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelError {
    Missing,
    /// The name has more than `max` characters.
    TooLong {
        max: usize,
    },
    /// Another label already has the name.
    Taken,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(name: &str) -> LabelForm {
        LabelForm {
            name: name.to_owned(),
            color: LabelColor::Gray,
        }
    }

    #[test]
    fn tidies_and_checks_names() {
        assert_eq!(form("  performance ").name().as_deref(), Ok("performance"));
        assert_eq!(form(" ").name(), Err(LabelError::Missing));
        assert!(form(&"a".repeat(MAX_NAME + 1)).name().is_err());
    }

    #[test]
    fn finds_the_labels_of_an_issue() {
        let label = |id, name: &str| Label {
            id,
            name: name.to_owned(),
            color: LabelColor::Gray,
        };
        let labels = IssueLabels {
            all: vec![label(1, "api"), label(2, "design"), label(3, "ux")],
            by_issue: HashMap::from([(10, vec![3, 1]), (11, vec![1])]),
        };
        let names = |issue| {
            labels
                .of(issue)
                .map(|label| label.name.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(10), ["api", "ux"]);
        assert_eq!(names(12), [] as [&str; 0]);
    }
}
