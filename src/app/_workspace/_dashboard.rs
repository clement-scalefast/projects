//! The `_dashboard` group puts this page at `/`, under the workspace layout.

use topcoat::{
    Result,
    context::Cx,
    l10n::{Translation, fluent::messages},
    router::{href, page},
    runtime::link,
    view::{StaticClass, View, attributes, class, component, element_id, view},
};

use super::{
    Section,
    issues::issue_table,
    projects::{project_code, project_id},
    team::user_id,
};
use crate::{
    components::{
        card::{card, card_content, card_description, card_header, card_title},
        chart::{bar_chart, line_chart},
        progress::progress,
    },
    features::{auth::require_user, issue::Issue, project::Project, stats::Workload},
};

messages!("{mod}/l10n/{locale}.ftl");

/// How many of the user's issues the dashboard lists.
const MY_ISSUES: usize = 8;

/// Chart values run from oldest to newest.
enum Chart<'a> {
    Bars(&'a [i64]),
    Line(&'a [i64]),
}

#[page(with(Section::Dashboard))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let first_name = user.name.split_whitespace().next().unwrap_or(&user.name);
    let workload = Workload::load(cx).await?;
    let mine = Issue::assigned_to(cx, user.id, false).await?;
    let projects = Project::list(cx).await?;

    Ok(view! {
        <header class="flex flex-col gap-1">
            <h1 class="text-2xl font-semibold tracking-tight">
                (t!(welcome, name: first_name))
            </h1>
            <p class="text-muted-foreground">(t!(welcome.description))</p>
        </header>

        <section class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
            stat_card(
                label: t!(open_issues),
                value: workload.open,
                note: t!(open_issues.note, count: workload.in_progress),
                accent: class!("text-sky-500"),
                chart: Some(Chart::Line(&workload.open_by_day))
            )
            stat_card(
                label: t!(created),
                value: workload.created.this_week,
                note: t!(week_before, count: workload.created.last_week),
                accent: class!("text-violet-500"),
                chart: Some(Chart::Bars(&workload.created_by_day))
            )
            stat_card(
                label: t!(completed),
                value: workload.completed.this_week,
                note: t!(week_before, count: workload.completed.last_week),
                accent: class!("text-emerald-500"),
                chart: Some(Chart::Bars(&workload.completed_by_day))
            )
            stat_card(
                label: t!(overdue),
                value: workload.overdue,
                note: t!(overdue.note),
                accent: class!("text-rose-500"),
                chart: None
            )
        </section>

        <div class="grid grid-cols-1 gap-6 lg:grid-cols-3 lg:items-start">
            card(
                attrs: attributes! { class="overflow-hidden pb-0 lg:col-span-2" },
                card_header(
                    card_title((t!(my_issues)))
                    card_description((t!(my_issues.description, count: mine.len())))
                )
                if mine.is_empty() {
                    <p class="px-6 pb-6 text-sm text-muted-foreground">
                        (t!(no_issues))
                    </p>
                } else {
                    issue_table(issues: &mine[..mine.len().min(MY_ISSUES)])
                    if mine.len() > MY_ISSUES {
                        <p class="border-t border-border px-6 py-3 text-sm">
                            link(
                                href: href!(user_id::page, user_id: user.id),
                                attrs: attributes! { class="font-medium hover:underline" },
                                (t!(view_all, count: mine.len()))
                            )
                        </p>
                    }
                }
            )

            card(
                card_header(
                    card_title((t!(projects)))
                    card_description((t!(projects.description)))
                )
                card_content(
                    <ul class="flex flex-col gap-5">
                        #[key(project.id)]
                        for project in &projects {
                            project_progress(project: project)
                        }
                    </ul>
                )
            )
        </div>
    })
}

/// Shows a figure with a note below it and a chart of its recent history.
/// The chart uses `accent` as its color.
#[component]
async fn stat_card(
    cx: &Cx,
    label: Translation<'_>,
    value: usize,
    note: Translation<'_>,
    accent: StaticClass,
    chart: Option<Chart<'_>>,
) -> Result<impl View> {
    Ok(view! {
        card(
            attrs: attributes! { class="gap-0 overflow-hidden pb-0" },
            <p class="px-6 text-sm text-muted-foreground">(label)</p>
            <p class="mt-1 px-6 text-3xl font-semibold tracking-tight tabular-nums">
                (value)
            </p>
            <p class="mt-2 px-6 text-xs text-muted-foreground">(note)</p>
            <div class=(class!("mt-4 h-16", accent))>
                match chart {
                    Some(Chart::Bars(values)) => bar_chart(values: values),
                    Some(Chart::Line(values)) => line_chart(values: values),
                    None => {

                    }
                }
            </div>
        )
    })
}

/// A project's name and how much of it is done, linked to its board. The issues
/// must already be loaded.
#[component]
async fn project_progress(cx: &Cx, project: &Project) -> Result<impl View> {
    let label_id = element_id(cx);
    let done = project.progress();

    Ok(view! {
        <li class="relative flex flex-col gap-2 text-sm">
            <div class="flex items-center gap-2">
                project_code(code: &project.code)
                link(
                    href: href!(project_id::page, project_id: project.id),
                    attrs: attributes! {
                        id=(&label_id)
                        class="truncate font-medium after:absolute after:inset-0 \
                            hover:underline"
                    },
                    (&project.name)
                )
                <span class="ml-auto shrink-0 text-muted-foreground tabular-nums">
                    (done.done)
                    "/"
                    (done.total)
                </span>
            </div>
            progress(
                value: f32::from(done.percent()),
                attrs: attributes! { aria-labelledby=(&label_id) }
            )
        </li>
    })
}
