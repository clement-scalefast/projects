pub(super) mod issue_id;
pub(super) mod new;

use jiff::{Timestamp, civil::Date};
use serde::Serialize;
use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    l10n::{DateTime, DateTimeStyle, Translation, fluent::messages},
    router::{Slot, href, layout, page, query_params},
    runtime::{Event, Signal, link, shard, signal},
    view::{Child, StaticClass, View, attributes, class, component, element_id, view},
};

use super::Section;
use crate::{
    app::_common::time_ago,
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        badge::{BadgeVariant, badge},
        button::{ButtonSize, ButtonVariant, button, button_variants},
        field::{field, field_error, field_group, field_label},
        input::input,
        list_pagination::{ListPage, list_pagination},
        select::select,
        table::{table, table_body, table_cell, table_head, table_header, table_row},
        textarea::textarea,
    },
    features::{
        auth::require_user,
        issue::{
            Issue, IssueError, IssueErrors, IssueForm, IssueKind, IssueSearch, IssueSort,
            IssueStatus, Priority,
        },
        project::Project,
        stats::today,
        user::User,
    },
};

messages!("{mod}/l10n/{locale}.ftl");

const PER_PAGE: usize = 20;

/// URL parameters for the initial list. Pagination clicks update a signal without
/// changing the URL; opening a page link in a new tab uses these parameters.
#[query_params(error = redirect("?"))]
#[derive(Serialize)]
pub struct IssuesQuery {
    q: Option<String>,
    project: Option<u64>,
    status: Option<IssueStatus>,
    priority: Option<Priority>,
    sort: Option<IssueSort>,
    page: Option<usize>,
}

impl IssuesQuery {
    fn new(filter: &IssueSearch<'_>, sort: IssueSort, number: usize) -> Self {
        Self {
            q: (!filter.text.is_empty()).then(|| filter.text.to_owned()),
            project: filter.project,
            status: filter.status,
            priority: filter.priority,
            sort: (sort != IssueSort::default()).then_some(sort),
            page: (number > 1).then_some(number),
        }
    }
}

/// Marks the issue pages as part of the issues section.
#[layout(with(Section::Issues))]
async fn section(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { (slot) })
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let query = query_params::<IssuesQuery>(cx)?;
    let projects = Project::names(cx).await?;
    let initial_project = query.project.map_or_default(|id| id.to_string());
    let initial_status = query.status.map_or("", IssueStatus::slug);
    let initial_priority = query.priority.map_or("", Priority::slug);
    let initial_sort = query.sort.unwrap_or_default().slug();

    let search = signal(cx, || query.q.clone().unwrap_or_default());
    let project = signal(cx, || initial_project.clone());
    let status = signal(cx, || initial_status.to_owned());
    let priority = signal(cx, || initial_priority.to_owned());
    let sort = signal(cx, || initial_sort.to_owned());
    let page_number = signal(cx, || query.page.unwrap_or(1));
    let search_id = element_id(cx);
    let project_id = element_id(cx);
    let status_id = element_id(cx);
    let priority_id = element_id(cx);
    let sort_id = element_id(cx);

    Ok(view! {
        <header class="flex flex-wrap items-center gap-4">
            <div class="flex flex-col gap-1">
                <h1 class="text-2xl font-semibold tracking-tight">(t!(title))</h1>
                <p class="text-muted-foreground">(t!(title.description))</p>
            </div>
            if user.role.can_edit() {
                link(
                    href: href!(new::page),
                    attrs: attributes! {
                        class=(class!(
                            button_variants(ButtonVariant::Primary, ButtonSize::Md),
                            "ml-auto",
                        ))
                    },
                    icon(data: iconify_icon!("lucide:plus"))
                    (t!(new_issue))
                )
            }
        </header>

        <section
            class="overflow-hidden rounded-xl border border-border bg-card \
                text-card-foreground shadow-xs"
        >
            <div class="flex flex-col gap-2 p-3 lg:flex-row lg:items-center">
                <div class="relative lg:max-w-xs lg:flex-1">
                    <label for=(&search_id) class="sr-only">(t!(search))</label>
                    icon(
                        data: iconify_icon!("lucide:search"),
                        attrs: attributes! {
                            class="pointer-events-none absolute top-1/2 left-3 size-4 \
                                -translate-y-1/2 text-muted-foreground"
                        }
                    )
                    input(
                        attrs: attributes! {
                            id=(&search_id)
                            type="search"
                            placeholder=(t!(search.placeholder))
                            class="pl-9"
                            :value=$(search.get())
                            @input=$(|e: Event| {
                                search.set(e.target.value);
                                page_number.set(1);
                            })
                        }
                    )
                </div>
                <div class="grid grid-cols-2 gap-2 lg:ml-auto lg:flex">
                    <label for=(&project_id) class="sr-only">
                        (t!(project_filter))
                    </label>
                    select(
                        attrs: attributes! {
                            id=(&project_id)
                            class="lg:w-40"
                            :value=$(project.get())
                            @change=$(|e: Event| {
                                project.set(e.target.value);
                                page_number.set(1);
                            })
                        },
                        <option value="" selected=(initial_project.is_empty())>
                            (t!(project_filter.all))
                        </option>
                        for option in &projects {
                            <option
                                value=(option.id)
                                selected=(query.project == Some(option.id))
                            >
                                (&option.name)
                            </option>
                        }
                    )
                    <label for=(&status_id) class="sr-only">(t!(status_filter))</label>
                    select(
                        attrs: attributes! {
                            id=(&status_id)
                            class="lg:w-36"
                            :value=$(status.get())
                            @change=$(|e: Event| {
                                status.set(e.target.value);
                                page_number.set(1);
                            })
                        },
                        <option value="" selected=(initial_status.is_empty())>
                            (t!(status_filter.all))
                        </option>
                        for option in IssueStatus::ALL {
                            <option
                                value=(option.slug())
                                selected=(initial_status == option.slug())
                            >
                                (status_label(option))
                            </option>
                        }
                    )
                    <label for=(&priority_id) class="sr-only">
                        (t!(priority_filter))
                    </label>
                    select(
                        attrs: attributes! {
                            id=(&priority_id)
                            class="lg:w-36"
                            :value=$(priority.get())
                            @change=$(|e: Event| {
                                priority.set(e.target.value);
                                page_number.set(1);
                            })
                        },
                        <option value="" selected=(initial_priority.is_empty())>
                            (t!(priority_filter.all))
                        </option>
                        for option in Priority::ALL {
                            <option
                                value=(option.slug())
                                selected=(initial_priority == option.slug())
                            >
                                (priority_label(option))
                            </option>
                        }
                    )
                    <label for=(&sort_id) class="sr-only">(t!(sort))</label>
                    select(
                        attrs: attributes! {
                            id=(&sort_id)
                            class="lg:w-40"
                            :value=$(sort.get())
                            @change=$(|e: Event| {
                                sort.set(e.target.value);
                                page_number.set(1);
                            })
                        },
                        for option in IssueSort::ALL {
                            <option
                                value=(option.slug())
                                selected=(initial_sort == option.slug())
                            >
                                (sort_label(option))
                            </option>
                        }
                    )
                </div>
            </div>

            issue_list(
                search: search.clone(),
                project: project.clone(),
                status: status.clone(),
                priority: priority.clone(),
                sort: sort.clone(),
                page_number: page_number.clone()
            )
        </section>
    })
}

/// The choices of the filter menus, without the search text.
#[derive(Clone, Copy)]
struct Filters {
    project: Option<u64>,
    status: Option<IssueStatus>,
    priority: Option<Priority>,
}

impl Filters {
    fn search(self, text: &str) -> IssueSearch<'_> {
        IssueSearch {
            project: self.project,
            status: self.status,
            priority: self.priority,
            text,
        }
    }
}

#[shard]
async fn issue_list(
    cx: &Cx,
    search: Signal<String>,
    project: Signal<String>,
    status: Signal<String>,
    priority: Signal<String>,
    sort: Signal<String>,
    page_number: Signal<usize>,
) -> Result<impl View> {
    // Shards can be requested without the page layout, so they need their own sign-in check.
    require_user(cx).await?;

    let text = search.get();
    // The view takes ownership of `text`, so it builds its own search for page links.
    let filters = Filters {
        project: project.get().parse().ok(),
        status: status.get().parse().ok(),
        priority: priority.get().parse().ok(),
    };
    let sort = sort.get().parse::<IssueSort>().unwrap_or_default();
    let count = filters.search(&text).count(cx).await?;
    let shown = ListPage::new(count, PER_PAGE, page_number.get());
    let issues = if count == 0 {
        Vec::new()
    } else {
        filters
            .search(&text)
            .page(cx, sort, shown.number, PER_PAGE)
            .await?
    };

    Ok(view! {
        if count == 0 {
            <div class="flex flex-col items-center gap-3 px-6 py-16 text-center">
                <span
                    class="flex size-10 items-center justify-center rounded-full \
                        bg-foreground/5 text-muted-foreground"
                >
                    icon(data: iconify_icon!("lucide:search-x"))
                </span>
                <div class="flex flex-col gap-1">
                    <p class="font-medium">(t!(no_results))</p>
                    <p class="text-sm text-muted-foreground">
                        (t!(no_results.description))
                    </p>
                </div>
                button(
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        type="button"
                        @click=$(|_e: Event| {
                            search.set("".to_owned());
                            project.set("".to_owned());
                            status.set("".to_owned());
                            priority.set("".to_owned());
                            page_number.set(1);
                        })
                    },
                    (t!(clear_filters))
                )
            </div>
        } else {
            table(
                table_header(
                    table_row(
                        table_head((t!(column_key)))
                        table_head(
                            attrs: attributes! { class="w-full" },
                            (t!(column_title))
                        )
                        table_head((t!(column_status)))
                        table_head((t!(column_priority)))
                        table_head((t!(column_assignee)))
                        table_head((t!(column_updated)))
                    )
                )
                table_body(
                    for issue in &issues {
                        table_row(
                            attrs: attributes! { class="relative" },
                            table_cell(issue_key(issue: issue))
                            // The title takes the space the other columns leave and is cut off to fit.
                            table_cell(
                                attrs: attributes! { class="w-full max-w-0 min-w-48" },
                                issue_title(issue: issue)
                            )
                            table_cell(status_badge(status: issue.status))
                            table_cell(priority_badge(priority: issue.priority))
                            table_cell(assignee(user: issue.assignee.get().as_ref()))
                            table_cell(
                                attrs: attributes! { class="text-muted-foreground" },
                                time_ago(at: issue.updated_at)
                            )
                        )
                    }
                )
            )

            list_pagination(
                page: shown,
                href: |number| {
                    href!(page)
                        .query(&IssuesQuery::new(&filters.search(&text), sort, number))
                        .resolve(cx)
                },
                number: &page_number
            )
        }
    })
}

/// A table of issues without their assignees, for pages about one person.
/// The projects must already be loaded.
#[component]
pub(super) async fn issue_table(cx: &Cx, issues: &[Issue]) -> Result<impl View> {
    let today = today(Timestamp::now());

    Ok(view! {
        table(
            table_header(
                table_row(
                    table_head((t!(column_key)))
                    table_head(
                        attrs: attributes! { class="w-full" },
                        (t!(column_title))
                    )
                    table_head((t!(column_status)))
                    table_head((t!(column_priority)))
                    table_head((t!(column_due)))
                )
            )
            table_body(
                for issue in issues {
                    table_row(
                        attrs: attributes! { class="relative" },
                        table_cell(issue_key(issue: issue))
                        // The title takes the space the other columns leave and is cut off to fit.
                        table_cell(
                            attrs: attributes! { class="w-full max-w-0 min-w-48" },
                            issue_title(issue: issue)
                        )
                        table_cell(status_badge(status: issue.status))
                        table_cell(priority_badge(priority: issue.priority))
                        table_cell(
                            if let Some(due) = issue.due_on {
                                due_date(due: due, overdue: issue.is_overdue(today))
                            } else {
                                <span class="text-muted-foreground">"-"</span>
                            }
                        )
                    )
                }
            )
        )
    })
}

/// The issue key as a link that covers its table row.
#[component]
async fn issue_key(cx: &Cx, issue: &Issue) -> Result<impl View> {
    Ok(view! {
        link(
            href: href!(issue_id::page, issue_id: issue.id),
            attrs: attributes! {
                class="font-mono text-xs font-medium whitespace-nowrap text-muted-foreground \
                    after:absolute after:inset-0"
            },
            (issue.key())
        )
    })
}

#[component]
async fn issue_title(cx: &Cx, issue: &Issue) -> Result<impl View> {
    Ok(view! {
        <div class="flex min-w-0 items-center gap-2">
            kind_icon(kind: issue.kind)
            <span class="truncate font-medium" title=(&issue.title)>
                (&issue.title)
            </span>
        </div>
    })
}

#[component]
pub(super) async fn status_badge(cx: &Cx, status: IssueStatus) -> Result<impl View> {
    Ok(view! {
        badge(
            variant: BadgeVariant::Outline,
            attrs: attributes! { class="whitespace-nowrap" },
            status_dot(status: status)
            (status_label(status))
        )
    })
}

#[component]
pub(super) async fn status_dot(cx: &Cx, status: IssueStatus) -> Result<impl View> {
    let dot: StaticClass = match status {
        IssueStatus::Backlog => class!("bg-muted-foreground/60"),
        IssueStatus::Todo => class!("bg-sky-500"),
        IssueStatus::InProgress => class!("bg-amber-500"),
        IssueStatus::InReview => class!("bg-violet-500"),
        IssueStatus::Done => class!("bg-emerald-500"),
    };

    Ok(view! { <span class=(class!("size-1.5 shrink-0 rounded-full", dot))></span> })
}

/// An arrow whose direction and color show the priority.
#[component]
pub(super) async fn priority_icon(cx: &Cx, priority: Priority) -> Result<impl View> {
    let (data, color): (_, StaticClass) = match priority {
        Priority::Urgent => (iconify_icon!("lucide:chevrons-up"), class!("text-rose-600")),
        Priority::High => (iconify_icon!("lucide:chevron-up"), class!("text-amber-600")),
        Priority::Medium => (iconify_icon!("lucide:equal"), class!("text-sky-600")),
        Priority::Low => (
            iconify_icon!("lucide:chevron-down"),
            class!("text-muted-foreground"),
        ),
    };

    Ok(view! {
        <span
            title=(priority_label(priority))
            class=(class!("inline-flex shrink-0", color))
        >
            icon(data: data, attrs: attributes! { class="size-4" })
            <span class="sr-only">(priority_label(priority))</span>
        </span>
    })
}

#[component]
pub(super) async fn priority_badge(cx: &Cx, priority: Priority) -> Result<impl View> {
    Ok(view! {
        <span class="inline-flex items-center gap-1.5 whitespace-nowrap">
            <span aria-hidden="true" class="inline-flex">
                priority_icon(priority: priority)
            </span>
            (priority_label(priority))
        </span>
    })
}

/// A colored icon for the kind of issue, named in its tooltip.
#[component]
pub(super) async fn kind_icon(cx: &Cx, kind: IssueKind) -> Result<impl View> {
    let (data, color): (_, StaticClass) = match kind {
        IssueKind::Task => (iconify_icon!("lucide:square-check"), class!("text-sky-600")),
        IssueKind::Bug => (iconify_icon!("lucide:bug"), class!("text-rose-600")),
        IssueKind::Feature => (iconify_icon!("lucide:sparkles"), class!("text-violet-600")),
    };

    Ok(view! {
        <span title=(kind_label(kind)) class=(class!("inline-flex shrink-0", color))>
            icon(data: data, attrs: attributes! { class="size-4" })
            <span class="sr-only">(kind_label(kind))</span>
        </span>
    })
}

/// A due date, highlighted when it has passed.
#[component]
pub(super) async fn due_date(cx: &Cx, due: Date, overdue: bool) -> Result<impl View> {
    Ok(view! {
        <time
            datetime=(due.to_string())
            class=(class!(
                "inline-flex items-center gap-1 whitespace-nowrap",
                "font-medium text-rose-600" if overdue else "text-muted-foreground",
            ))
        >
            icon(
                data: iconify_icon!("lucide:calendar"),
                attrs: attributes! { class="size-3.5" }
            )
            (DateTime::from(due).date_style(DateTimeStyle::Medium))
            if overdue {
                <span class="sr-only">(t!(overdue))</span>
            }
        </time>
    })
}

/// A person's avatar and name, or a placeholder for nobody.
#[component]
pub(super) async fn assignee(cx: &Cx, user: Option<&User>) -> Result<impl View> {
    Ok(view! {
        if let Some(user) = user {
            <span class="flex items-center gap-2 whitespace-nowrap">
                avatar(size: AvatarSize::Sm, avatar_fallback((user.initials())))
                (&user.name)
            </span>
        } else {
            <span class="whitespace-nowrap text-muted-foreground">
                (t!(unassigned))
            </span>
        }
    })
}

pub(super) fn status_label(status: IssueStatus) -> Translation<'static> {
    match status {
        IssueStatus::Backlog => t!(status_backlog),
        IssueStatus::Todo => t!(status_todo),
        IssueStatus::InProgress => t!(status_in_progress),
        IssueStatus::InReview => t!(status_in_review),
        IssueStatus::Done => t!(status_done),
    }
}

pub(super) fn priority_label(priority: Priority) -> Translation<'static> {
    match priority {
        Priority::Urgent => t!(priority_urgent),
        Priority::High => t!(priority_high),
        Priority::Medium => t!(priority_medium),
        Priority::Low => t!(priority_low),
    }
}

pub(super) fn kind_label(kind: IssueKind) -> Translation<'static> {
    match kind {
        IssueKind::Task => t!(kind_task),
        IssueKind::Bug => t!(kind_bug),
        IssueKind::Feature => t!(kind_feature),
    }
}

fn sort_label(sort: IssueSort) -> Translation<'static> {
    match sort {
        IssueSort::Updated => t!(sort_updated),
        IssueSort::Newest => t!(sort_newest),
        IssueSort::Oldest => t!(sort_oldest),
        IssueSort::Priority => t!(sort_priority),
    }
}

/// The fields for creating or editing an issue, to place inside a `<form>`.
/// Pass `projects` to let the user choose the project, or `None` when it is fixed.
#[component]
async fn issue_fields(
    cx: &Cx,
    form: &IssueForm,
    errors: IssueErrors,
    projects: Option<&[Project]>,
    users: &[User],
) -> Result<impl View> {
    let project_id = element_id(cx);
    let title_id = element_id(cx);
    let description_id = element_id(cx);
    let kind_id = element_id(cx);
    let status_id = element_id(cx);
    let priority_id = element_id(cx);
    let assignee_id = element_id(cx);
    let due_id = element_id(cx);

    Ok(view! {
        field_group(
            if let Some(projects) = projects {
                labeled(
                    label: t!(field_project),
                    for_id: &project_id,
                    error: errors.project.map(error_message),
                    select(
                        attrs: attributes! {
                            id=(&project_id)
                            name="project"
                            aria-invalid=(errors.project.is_some().then_some("true"))
                        },
                        <option value="" selected=(form.project.is_empty())>
                            (t!(field_project.choose))
                        </option>
                        for project in projects {
                            <option
                                value=(project.id)
                                selected=(form.project == project.id.to_string())
                            >
                                (&project.name)
                            </option>
                        }
                    )
                )
            }
            labeled(
                label: t!(field_title),
                for_id: &title_id,
                error: errors.title.map(error_message),
                input(
                    attrs: attributes! {
                        id=(&title_id)
                        name="title"
                        value=(&form.title)
                        autocomplete="off"
                        aria-invalid=(errors.title.is_some().then_some("true"))
                    }
                )
            )
            labeled(
                label: t!(field_description),
                for_id: &description_id,
                error: errors.description.map(error_message),
                textarea(
                    attrs: attributes! {
                        id=(&description_id)
                        name="description"
                        class="min-h-32"
                        placeholder=(t!(field_description.placeholder))
                        aria-invalid=(errors.description.is_some().then_some("true"))
                    },
                    (&form.description)
                )
            )
            <div class="grid grid-cols-1 gap-6 sm:grid-cols-3">
                labeled(
                    label: t!(field_kind),
                    for_id: &kind_id,
                    error: None,
                    select(
                        attrs: attributes! { id=(&kind_id) name="kind" },
                        for kind in IssueKind::ALL {
                            <option value=(kind.slug()) selected=(form.kind == kind)>
                                (kind_label(kind))
                            </option>
                        }
                    )
                )
                labeled(
                    label: t!(field_status),
                    for_id: &status_id,
                    error: None,
                    select(
                        attrs: attributes! { id=(&status_id) name="status" },
                        for status in IssueStatus::ALL {
                            <option
                                value=(status.slug())
                                selected=(form.status == status)
                            >
                                (status_label(status))
                            </option>
                        }
                    )
                )
                labeled(
                    label: t!(field_priority),
                    for_id: &priority_id,
                    error: None,
                    select(
                        attrs: attributes! { id=(&priority_id) name="priority" },
                        for priority in Priority::ALL {
                            <option
                                value=(priority.slug())
                                selected=(form.priority == priority)
                            >
                                (priority_label(priority))
                            </option>
                        }
                    )
                )
            </div>
            <div class="grid grid-cols-1 gap-6 sm:grid-cols-2">
                labeled(
                    label: t!(field_assignee),
                    for_id: &assignee_id,
                    error: errors.assignee.map(error_message),
                    select(
                        attrs: attributes! {
                            id=(&assignee_id)
                            name="assignee"
                            aria-invalid=(errors.assignee.is_some().then_some("true"))
                        },
                        <option value="" selected=(form.assignee.is_empty())>
                            (t!(unassigned))
                        </option>
                        for user in users {
                            <option
                                value=(user.id)
                                selected=(form.assignee == user.id.to_string())
                            >
                                (&user.name)
                            </option>
                        }
                    )
                )
                labeled(
                    label: t!(field_due),
                    for_id: &due_id,
                    error: errors.due_on.map(error_message),
                    input(
                        attrs: attributes! {
                            id=(&due_id)
                            name="due_on"
                            type="date"
                            value=(&form.due_on)
                            aria-invalid=(errors.due_on.is_some().then_some("true"))
                        }
                    )
                )
            </div>
        )
    })
}

/// A form control with its label and an optional error below it.
#[component]
pub(super) async fn labeled(
    cx: &Cx,
    label: Translation<'_>,
    for_id: &str,
    error: Option<Translation<'_>>,
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        field(
            field_label(attrs: attributes! { for=(for_id) }, (label))
            (child)
            if let Some(error) = error {
                field_error((error))
            }
        )
    })
}

fn error_message(error: IssueError) -> Translation<'static> {
    match error {
        IssueError::ProjectMissing => t!(error_project_missing),
        IssueError::TitleMissing => t!(error_title_missing),
        IssueError::TitleTooLong { max } => t!(error_title_too_long, max),
        IssueError::DescriptionTooLong { max } => t!(error_description_too_long, max),
        IssueError::AssigneeUnknown => t!(error_assignee_unknown),
        IssueError::DueDateInvalid => t!(error_due_invalid),
    }
}
