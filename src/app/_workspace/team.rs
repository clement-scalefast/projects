pub(super) mod user_id;

use topcoat::{
    Result,
    context::Cx,
    l10n::fluent::messages,
    router::{Slot, href, layout, page},
    runtime::link,
    view::{View, attributes, view},
};

use super::Section;
use crate::{
    app::_common::role_label,
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        badge::{BadgeVariant, badge},
        card::card,
        table::{table, table_body, table_cell, table_head, table_header, table_row},
    },
    features::{auth::require_user, issue::Issue, project::Progress, user::User},
};

messages!("{mod}/l10n/{locale}.ftl");

/// Marks the team pages as part of the team section.
#[layout(with(Section::Team))]
async fn section(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { (slot) })
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let users = User::list(cx).await?;
    let issues = Issue::all_plain(cx).await?;

    Ok(view! {
        <header class="flex flex-col gap-1">
            <h1 class="text-2xl font-semibold tracking-tight">(t!(title))</h1>
            <p class="text-muted-foreground">(t!(title.description))</p>
        </header>

        card(
            attrs: attributes! { class="overflow-hidden py-0" },
            table(
                table_header(
                    table_row(
                        table_head(
                            attrs: attributes! { class="w-full" },
                            (t!(column_member))
                        )
                        table_head((t!(column_role)))
                        table_head(
                            attrs: attributes! { class="text-right" },
                            (t!(column_open))
                        )
                        table_head(
                            attrs: attributes! { class="text-right" },
                            (t!(column_done))
                        )
                    )
                )
                table_body(
                    for user in &users {
                        let work = workload(&issues, user.id);
                        table_row(
                            attrs: attributes! { class="relative" },
                            table_cell(
                                <div class="flex items-center gap-3">
                                    avatar(
                                        size: AvatarSize::Sm,
                                        avatar_fallback((user.initials()))
                                    )
                                    <div class="flex max-w-64 min-w-0 flex-col">
                                        link(
                                            href: href!(user_id::page, user_id: user.id),
                                            attrs: attributes! {
                                                class="truncate font-medium after:absolute \
                                                    after:inset-0"
                                            },
                                            (&user.name)
                                        )
                                        <span class="truncate text-xs text-muted-foreground">
                                            (&user.email)
                                        </span>
                                    </div>
                                </div>
                            )
                            table_cell(
                                badge(
                                    variant: BadgeVariant::Outline,
                                    (role_label(user.role))
                                )
                            )
                            table_cell(
                                attrs: attributes! { class="text-right font-medium tabular-nums" },
                                (work.open())
                            )
                            table_cell(
                                attrs: attributes! { class="text-right text-muted-foreground tabular-nums" },
                                (work.done)
                            )
                        )
                    }
                )
            )
        )
    })
}

/// Counts the issues assigned to a user.
fn workload(issues: &[Issue], user_id: u64) -> Progress {
    Progress::of(
        issues
            .iter()
            .filter(|issue| issue.assignee_id == Some(user_id))
            .map(|issue| issue.status),
    )
}
