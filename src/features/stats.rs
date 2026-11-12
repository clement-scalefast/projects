//! Workspace figures for the dashboard, calculated from the issues in the database.
//! Days are UTC calendar days.

use jiff::{Timestamp, civil::Date, tz::TimeZone};
use topcoat::{Result, context::Cx};

use super::issue::{Issue, IssueStatus};

const DAY: i64 = 24 * 60 * 60;

/// Days of history in the charts, including today.
const HISTORY: usize = 14;

/// Days counted as "this week", including today.
const WEEK: usize = 7;

/// The parts of an issue the figures depend on.
#[derive(Debug, Clone, Copy)]
struct Snapshot {
    status: IssueStatus,
    created_at: Timestamp,
    completed_at: Option<Timestamp>,
    due_on: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workload {
    /// Issues that are not done.
    pub open: usize,
    pub in_progress: usize,
    /// Open issues whose due date has passed.
    pub overdue: usize,
    /// Issues created in the last seven days, and in the seven days before.
    pub created: Trend,
    /// Issues completed in the last seven days, and in the seven days before.
    pub completed: Trend,
    /// Open issues at the end of each day, oldest first. The last entry is today.
    pub open_by_day: Vec<i64>,
    /// Issues created on each day, oldest first. The last entry is today.
    pub created_by_day: Vec<i64>,
    /// Issues completed on each day, oldest first. The last entry is today.
    pub completed_by_day: Vec<i64>,
}

/// A count for the last seven days and for the seven days before.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Trend {
    pub this_week: usize,
    pub last_week: usize,
}

impl Workload {
    pub async fn load(cx: &Cx) -> Result<Self> {
        let issues = Issue::all_plain(cx).await?;
        let snapshots = issues
            .iter()
            .map(|issue| Snapshot {
                status: issue.status,
                created_at: issue.created_at,
                completed_at: issue.completed_at,
                due_on: issue.due_on,
            })
            .collect::<Vec<_>>();
        Ok(Self::of(&snapshots, Timestamp::now()))
    }

    fn of(issues: &[Snapshot], now: Timestamp) -> Self {
        let today = today(now);
        // The first second of tomorrow.
        let end = (now.as_second().div_euclid(DAY) + 1) * DAY;
        // Index into the charts for a time, where the last index is today.
        let day = |at: Timestamp| {
            let ago = usize::try_from((end - 1 - at.as_second()).div_euclid(DAY)).ok()?;
            (HISTORY - 1).checked_sub(ago)
        };

        let mut created_by_day = vec![0; HISTORY];
        let mut completed_by_day = vec![0; HISTORY];
        let mut open_by_day = vec![0; HISTORY];
        for issue in issues {
            if let Some(index) = day(issue.created_at) {
                created_by_day[index] += 1;
            }
            if let Some(index) = issue.completed_at.and_then(day) {
                completed_by_day[index] += 1;
            }
            for (index, open) in open_by_day.iter_mut().enumerate() {
                let ago = i64::try_from(HISTORY - 1 - index).unwrap_or(i64::MAX);
                let day_end = end - ago * DAY;
                let created = issue.created_at.as_second() < day_end;
                let completed = issue
                    .completed_at
                    .is_some_and(|at| at.as_second() < day_end);
                if created && !completed {
                    *open += 1;
                }
            }
        }

        let open = issues
            .iter()
            .filter(|issue| issue.status != IssueStatus::Done);
        Self {
            open: open.clone().count(),
            in_progress: issues
                .iter()
                .filter(|issue| issue.status == IssueStatus::InProgress)
                .count(),
            overdue: open
                .filter(|issue| issue.due_on.is_some_and(|due| due < today))
                .count(),
            created: Trend::of(&created_by_day),
            completed: Trend::of(&completed_by_day),
            open_by_day,
            created_by_day,
            completed_by_day,
        }
    }
}

impl Trend {
    fn of(by_day: &[i64]) -> Self {
        let (before, week) = by_day.split_at(by_day.len() - WEEK);
        let total = |days: &[i64]| usize::try_from(days.iter().sum::<i64>()).unwrap_or(0);
        Self {
            this_week: total(week),
            last_week: total(&before[before.len().saturating_sub(WEEK)..]),
        }
    }
}

/// The current UTC calendar day.
pub fn today(now: Timestamp) -> Date {
    now.to_zoned(TimeZone::UTC).date()
}

#[cfg(test)]
mod tests {
    use jiff::SignedDuration;

    use super::*;

    fn noon() -> Timestamp {
        "2026-11-12T12:00:00Z".parse().unwrap()
    }

    fn days(count: i64) -> SignedDuration {
        SignedDuration::from_hours(24 * count)
    }

    fn issue(created_days_ago: i64, completed_days_ago: Option<i64>) -> Snapshot {
        Snapshot {
            status: if completed_days_ago.is_some() {
                IssueStatus::Done
            } else {
                IssueStatus::Todo
            },
            created_at: noon() - days(created_days_ago),
            completed_at: completed_days_ago.map(|ago| noon() - days(ago)),
            due_on: None,
        }
    }

    #[test]
    fn counts_issues_on_the_day_they_happen() {
        let workload = Workload::of(
            &[issue(0, None), issue(3, Some(1)), issue(30, None)],
            noon(),
        );
        assert_eq!(workload.created_by_day[HISTORY - 1], 1);
        assert_eq!(workload.created_by_day[HISTORY - 4], 1);
        assert_eq!(workload.created_by_day.iter().sum::<i64>(), 2);
        assert_eq!(workload.completed_by_day[HISTORY - 2], 1);
        assert_eq!(workload.completed_by_day.iter().sum::<i64>(), 1);
    }

    #[test]
    fn open_issues_end_at_the_current_count() {
        let issues = [issue(0, None), issue(3, Some(1)), issue(30, None)];
        let workload = Workload::of(&issues, noon());
        assert_eq!(workload.open, 2);
        assert_eq!(workload.open_by_day.last(), Some(&2));
        // Three days ago, the completed issue was still open and today's did not exist.
        assert_eq!(workload.open_by_day[HISTORY - 4], 2);
        assert_eq!(workload.open_by_day[0], 1);
    }

    #[test]
    fn compares_this_week_with_the_week_before() {
        let issues = [
            issue(0, None),
            issue(6, None),
            issue(7, None),
            issue(13, None),
        ];
        let workload = Workload::of(&issues, noon());
        assert_eq!(
            workload.created,
            Trend {
                this_week: 2,
                last_week: 2
            }
        );
    }

    #[test]
    fn only_open_issues_are_overdue() {
        let yesterday = today(noon() - days(1));
        let late = |status| Snapshot {
            status,
            due_on: Some(yesterday),
            ..issue(5, None)
        };
        let due_today = Snapshot {
            due_on: Some(today(noon())),
            ..issue(5, None)
        };
        let workload = Workload::of(
            &[
                late(IssueStatus::InProgress),
                late(IssueStatus::Done),
                due_today,
            ],
            noon(),
        );
        assert_eq!(workload.overdue, 1);
        assert_eq!(workload.in_progress, 1);
    }
}
