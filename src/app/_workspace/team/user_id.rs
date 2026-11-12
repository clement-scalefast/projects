use topcoat::{
    Result,
    context::{Cx, memoize},
    icon::{icon, iconify::iconify_icon},
    l10n::Translation,
    router::{error::RouterErrorExt, module_param, page},
    view::{Child, View, attributes, component, view},
};

use super::t;
use crate::{
    app::{
        _common::role_label,
        _workspace::{Crumb, issues::issue_table},
    },
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        badge::{BadgeVariant, badge},
        card::{card, card_description, card_header, card_title},
    },
    features::{
        auth::require_user,
        issue::{Issue, IssueStatus},
        project::Progress,
        user::User,
    },
};

module_param!(pub user_id: u64 else not_found);

#[memoize(as_ref)]
async fn member(cx: &Cx) -> Result<User> {
    Ok(User::find(cx, *user_id(cx)?).await?.ok_or_not_found()?)
}

#[page(with(Crumb::new(async |cx| Ok(member(cx).await?.name.clone()))))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let member = member(cx).await?;
    let issues = Issue::assigned_to(cx, member.id, true).await?;
    let work = Progress::of(issues.iter().map(|issue| issue.status));
    let in_progress = issues
        .iter()
        .filter(|issue| issue.status == IssueStatus::InProgress)
        .count();

    Ok(view! {
        <header class="flex items-center gap-4">
            avatar(size: AvatarSize::Lg, avatar_fallback((member.initials())))
            <div class="flex min-w-0 flex-col gap-1">
                <div class="flex items-center gap-3">
                    <h1 class="truncate text-2xl font-semibold tracking-tight">
                        (&member.name)
                    </h1>
                    badge(variant: BadgeVariant::Outline, (role_label(member.role)))
                </div>
                <a
                    href=(format!("mailto:{}", member.email))
                    class="flex items-center gap-1.5 text-sm text-muted-foreground \
                        hover:underline"
                >
                    icon(
                        data: iconify_icon!("lucide:mail"),
                        attrs: attributes! { class="size-4" }
                    )
                    (&member.email)
                </a>
            </div>
        </header>

        <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
            stat(label: t!(stat_open), (work.open()))
            stat(label: t!(stat_in_progress), (in_progress))
            stat(label: t!(stat_done), (work.done))
        </div>

        card(
            attrs: attributes! { class="overflow-hidden pb-0" },
            card_header(
                card_title((t!(assigned)))
                card_description((t!(assigned.description)))
            )
            if issues.is_empty() {
                <p class="px-6 pb-6 text-sm text-muted-foreground">
                    (t!(nothing_assigned))
                </p>
            } else {
                issue_table(issues: &issues)
            }
        )
    })
}

#[component]
async fn stat(cx: &Cx, label: Translation<'_>, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        card(
            attrs: attributes! { class="gap-1" },
            <p class="px-6 text-sm text-muted-foreground">(label)</p>
            <div class="px-6 text-2xl font-semibold tracking-tight tabular-nums">
                (child)
            </div>
        )
    })
}
