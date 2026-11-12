use jiff::{Timestamp, civil::Date};
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, memoize, try_context},
    icon::{icon, iconify::iconify_icon},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{RouterErrorExt, SeeOther, rewrite, see_other},
        href, module_param, page, route,
    },
    runtime::link,
    view::{View, attributes, class, component, element_id, view},
};

use super::{project_code, project_fields, t};
use crate::{
    app::_workspace::{
        Crumb,
        issues::{self, due_date, issue_id, kind_icon, priority_icon, status_dot, status_label},
    },
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        button::{ButtonSize, ButtonVariant, button, button_variants},
        card::{card, card_content, card_footer},
    },
    features::{
        auth::require_user,
        issue::{Issue, IssueStatus},
        project::{Project, ProjectErrors, ProjectForm},
        stats::today,
        user::User,
    },
};

module_param!(pub project_id: u64 else not_found);

/// Remembers what is being dragged. The second entry lets a column ignore a card
/// that is dropped where it came from.
const DRAG_START: &str = "event.dataTransfer.setData('text/plain', this.dataset.issue); \
    event.dataTransfer.setData('text/x-status', this.dataset.status); \
    event.dataTransfer.effectAllowed = 'move'";

/// Allows dropping on a column and highlights it.
const DRAG_OVER: &str = "event.preventDefault(); this.dataset.over = 'true'";

/// Removes the highlight once the pointer leaves the column, not just one of its cards.
const DRAG_LEAVE: &str = "if (!this.contains(event.relatedTarget)) delete this.dataset.over";

/// Submits the column's hidden form to move the dropped card here.
const DROP: &str = "event.preventDefault(); delete this.dataset.over; \
    const issue = event.dataTransfer.getData('text/plain'); \
    const from = event.dataTransfer.getData('text/x-status'); \
    const form = this.querySelector('form[data-drop]'); \
    if (form && issue && from !== this.dataset.status) { \
        form.elements.issue.value = issue; form.submit(); \
    }";

struct Rejected {
    form: ProjectForm,
    errors: ProjectErrors,
}

#[memoize(as_ref)]
async fn project(cx: &Cx) -> Result<Project> {
    Ok(Project::find(cx, *project_id(cx)?)
        .await?
        .ok_or_not_found()?)
}

#[page(with(Crumb::new(async |cx| Ok(project(cx).await?.name.clone()))))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let project = project(cx).await?;
    let issues = Issue::of_project(cx, project.id).await?;
    let can_edit = user.role.can_edit();
    let today = today(Timestamp::now());
    let new_issue = issues::new::NewIssueQuery::in_project(project.id);

    Ok(view! {
        <header class="flex flex-col gap-4 sm:flex-row sm:items-start">
            <div class="flex min-w-0 flex-col gap-1">
                <div class="flex items-center gap-3">
                    <h1 class="truncate text-2xl font-semibold tracking-tight">
                        (&project.name)
                    </h1>
                    project_code(code: &project.code)
                </div>
                <p class="text-muted-foreground">(&project.description)</p>
            </div>
            <div class="flex shrink-0 gap-2 sm:ml-auto">
                if user.role.can_manage() {
                    link(
                        href: href!(settings, project_id: project.id),
                        attrs: attributes! {
                            class=(button_variants(
                                ButtonVariant::Outline,
                                ButtonSize::Md,
                            ))
                        },
                        icon(data: iconify_icon!("lucide:settings"))
                        (t!(settings))
                    )
                }
                if can_edit {
                    link(
                        href: href!(issues::new::page).query(&new_issue),
                        attrs: attributes! {
                            class=(button_variants(
                                ButtonVariant::Primary,
                                ButtonSize::Md,
                            ))
                        },
                        icon(data: iconify_icon!("lucide:plus"))
                        (t!(new_issue))
                    )
                }
            </div>
        </header>

        // The board scrolls sideways on narrow screens, out to the edges of the page.
        <div class="-mx-6 overflow-x-auto px-6 pb-2">
            <div
                class="grid min-w-max grid-flow-col auto-cols-[17rem] items-start gap-3 \
                    xl:min-w-0 xl:auto-cols-fr"
            >
                for status in IssueStatus::ALL {
                    column(
                        project: project,
                        status: status,
                        issues: &issues,
                        can_edit: can_edit,
                        today: today
                    )
                }
            </div>
        </div>
    })
}

/// A board column with the cards of one status. Cards can be dropped on it.
#[component]
async fn column(
    cx: &Cx,
    project: &Project,
    status: IssueStatus,
    issues: &[Issue],
    can_edit: bool,
    today: Date,
) -> Result<impl View> {
    let cards = issues
        .iter()
        .filter(|issue| issue.status == status)
        .collect::<Vec<_>>();

    Ok(view! {
        <section
            class="flex min-w-0 flex-col gap-3 rounded-xl border border-border bg-foreground/3 \
                p-2 data-[over=true]:border-ring data-[over=true]:bg-foreground/8"
            data-status=(status.slug())
            ondragover=(can_edit.then_some(DRAG_OVER))
            ondragleave=(can_edit.then_some(DRAG_LEAVE))
            ondrop=(can_edit.then_some(DROP))
        >
            <h2 class="flex items-center gap-2 px-1 pt-1 text-sm font-medium">
                status_dot(status: status)
                (status_label(status))
                <span class="font-normal text-muted-foreground tabular-nums">
                    (cards.len())
                </span>
            </h2>
            if can_edit {
                <form
                    method="post"
                    action=(href!(move_issue, project_id: project.id))
                    data-drop=""
                    hidden=""
                >
                    <input type="hidden" name="status" value=(status.slug())>
                    <input type="hidden" name="issue" value="">
                </form>
            }
            if cards.is_empty() {
                <p
                    class="rounded-lg border border-dashed border-border px-3 py-6 text-center \
                        text-xs text-muted-foreground"
                >
                    (t!(empty_column))
                </p>
            } else {
                <ol class="flex flex-col gap-2">
                    for &issue in &cards {
                        issue_card(
                            project: project,
                            issue: issue,
                            can_edit: can_edit,
                            today: today
                        )
                    }
                </ol>
            }
        </section>
    })
}

/// A card that links to its issue. Members can drag it to another column or move it
/// one column at a time with its arrow buttons.
#[component]
async fn issue_card(
    cx: &Cx,
    project: &Project,
    issue: &Issue,
    can_edit: bool,
    today: Date,
) -> Result<impl View> {
    Ok(view! {
        <li
            class=(class!(
                "relative flex flex-col gap-2 rounded-lg border border-border bg-card p-3",
                "text-sm text-card-foreground shadow-xs hover:border-foreground/25",
                "cursor-grab active:cursor-grabbing" if can_edit,
            ))
            draggable=(can_edit.then_some("true"))
            data-issue=(issue.id)
            data-status=(issue.status.slug())
            ondragstart=(can_edit.then_some(DRAG_START))
        >
            <div
                class="flex min-h-5 items-center gap-1.5 text-xs text-muted-foreground"
            >
                kind_icon(kind: issue.kind)
                <span class="font-mono">
                    (&project.code)
                    "-"
                    (issue.number)
                </span>
                if can_edit {
                    // Raised above the card's link so the buttons stay clickable.
                    <form
                        method="post"
                        action=(href!(move_issue, project_id: project.id))
                        class="relative z-10 ml-auto flex"
                    >
                        <input type="hidden" name="issue" value=(issue.id)>
                        if let Some(previous) = issue.status.previous() {
                            move_button(to: previous, back: true)
                        }
                        if let Some(next) = issue.status.next() {
                            move_button(to: next, back: false)
                        }
                    </form>
                }
            </div>
            // Dragging starts on the card, not on the link that covers it.
            link(
                href: href!(issue_id::page, issue_id: issue.id),
                attrs: attributes! {
                    draggable="false"
                    class="leading-snug font-medium after:absolute after:inset-0"
                },
                (&issue.title)
            )
            <div class="flex min-h-6 items-center gap-2 text-xs">
                if let Some(user) = issue.assignee.get() {
                    <span title=(&user.name)>
                        avatar(size: AvatarSize::Sm, avatar_fallback((user.initials())))
                    </span>
                }
                if let Some(due) = issue.due_on {
                    due_date(due: due, overdue: issue.is_overdue(today))
                }
                <span class="ml-auto inline-flex">
                    priority_icon(priority: issue.priority)
                </span>
            </div>
        </li>
    })
}

/// A small arrow button that moves a card to a neighboring column.
#[component]
async fn move_button(cx: &Cx, to: IssueStatus, back: bool) -> Result<impl View> {
    let label = t!(move_to, status: status_label(to).resolve(cx).into_owned());

    Ok(view! {
        button(
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Icon,
            attrs: attributes! {
                type="submit"
                name="status"
                value=(to.slug())
                class="size-5 text-muted-foreground"
                aria-label=(label.clone())
                title=(label)
            },
            if back {
                icon(data: iconify_icon!("lucide:chevron-left"))
            } else {
                icon(data: iconify_icon!("lucide:chevron-right"))
            }
        )
    })
}

#[derive(Deserialize)]
pub struct Move {
    issue: u64,
    status: IssueStatus,
}

#[route(POST "./move")]
pub async fn move_issue(cx: &Cx, Form(form): Form<Move>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let id = *project_id(cx)?;
    let mut issue = Issue::find(cx, form.issue)
        .await?
        .filter(|issue| issue.project_id == id)
        .ok_or_not_found()?;
    issue.move_to(cx, form.status).await?;

    Ok(see_other(href!(page, project_id: id).resolve(cx)))
}

#[page("./settings", with(Crumb::new(async |cx| Ok(project(cx).await?.name.clone()))))]
pub async fn settings(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    user.role.can_manage().then_some(()).ok_or_forbidden()?;
    let project = project(cx).await?;

    let rejected = try_context::<Rejected>(cx);
    let form = rejected.map_or_else(
        || ProjectForm::of(project),
        |rejected| rejected.form.clone(),
    );
    let errors = rejected.map(|rejected| rejected.errors).unwrap_or_default();
    let users = User::list(cx).await?;
    let form_id = element_id(cx);

    Ok(view! {
        if rejected.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }

        <header class="flex flex-col gap-1">
            <h1 class="text-2xl font-semibold tracking-tight">(t!(settings))</h1>
            <p class="text-muted-foreground">(t!(settings.description))</p>
        </header>

        card(
            attrs: attributes! { class="max-w-2xl" },
            card_content(
                // Disable browser validation so the form shows the server's field errors.
                <form
                    id=(&form_id)
                    method="post"
                    action=(href!(save, project_id: project.id))
                    novalidate=""
                >
                    project_fields(form: &form, errors: errors, users: &users)
                </form>
            )
            card_footer(
                attrs: attributes! { class="justify-end" },
                link(
                    href: href!(page, project_id: project.id),
                    attrs: attributes! {
                        class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
                    },
                    (t!(cancel))
                )
                button(attrs: attributes! { type="submit" form=(&form_id) }, (t!(save)))
            )
        )
    })
}

#[route(POST "./settings")]
pub async fn save(cx: &Cx, Form(form): Form<ProjectForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_manage().then_some(()).ok_or_forbidden()?;

    let id = *project_id(cx)?;
    let mut project = Project::find(cx, id).await?.ok_or_not_found()?;
    if let Err(errors) = project.edit(cx, &form).await? {
        // Rewrite the POST to keep the submitted values and errors in the request context.
        return Err(
            rewrite(href!(settings, project_id: id).resolve(cx), Body::empty())
                .method(Method::GET)
                .with(Rejected { form, errors })
                .into(),
        );
    }

    Ok(see_other(href!(page, project_id: id).resolve(cx)))
}
