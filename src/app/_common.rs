//! Helpers shared by several pages. The `_common` group has no routes.

use jiff::Timestamp;
use topcoat::{
    Result,
    context::Cx,
    l10n::{DateTime, DateTimeStyle, Translation, fluent::messages},
    router::{
        error::{SeeOther, see_other},
        href,
    },
    view::{View, component, view},
};

use crate::features::user::Role;

messages!("{mod}/l10n/{locale}.ftl");

/// Formats the date and time of day of `at` in UTC, such as `Oct 12, 2026, 2:05 PM UTC`.
pub fn timestamp(at: Timestamp) -> Translation<'static> {
    t!(timestamp, at)
}

pub fn role_label(role: Role) -> Translation<'static> {
    match role {
        Role::Admin => t!(role_admin),
        Role::Staff => t!(role_member),
        Role::Viewer => t!(role_viewer),
    }
}

/// Shows recent timestamps as elapsed time, such as `3 hours ago`, inside a `<time>`
/// element whose tooltip has the exact time. After a week, shows a date such as
/// `Oct 12, 2026`.
#[component]
pub async fn time_ago(cx: &Cx, at: Timestamp) -> Result<impl View> {
    let minutes = Timestamp::now().duration_since(at).as_mins();

    Ok(view! {
        <time datetime=(at.to_string()) title=(timestamp(at))>
            if minutes < 1 {
                (t!(just_now))
            } else if minutes < 60 {
                (t!(minutes_ago, count: minutes))
            } else if minutes < 24 * 60 {
                (t!(hours_ago, count: minutes / 60))
            } else if minutes < 7 * 24 * 60 {
                (t!(days_ago, count: minutes / (24 * 60)))
            } else {
                (DateTime::from(at).date_style(DateTimeStyle::Medium))
            }
        </time>
    })
}

/// Redirects back to the page a form was submitted from.
///
/// Only redirects to local paths because the submitted path is untrusted.
/// Redirects to the dashboard instead of other paths.
pub fn see_back(cx: &Cx, path: &str) -> SeeOther {
    let local = path.starts_with('/') && !path.starts_with("//") && !path.contains('\\');
    if local {
        see_other(path)
    } else {
        see_other(href!(super::_workspace::_dashboard::page).resolve(cx))
    }
}
