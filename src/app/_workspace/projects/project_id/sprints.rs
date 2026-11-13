use jiff::civil::Date;
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_context},
    icon::{icon, iconify::iconify_icon},
    l10n::{DateTime, DateTimeStyle, Translation},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{RouterErrorExt, SeeOther, rewrite, see_other},
        href, page, path_param, route,
    },
    view::{View, attributes, component, element_id, view},
};

use super::{Tab, project, project_header, project_id};
use crate::{
    app::{
        _common::confirm_delete,
        _workspace::{
            Crumb,
            issues::{issue_key, issue_title, labeled, priority_icon, status_badge},
            projects::t,
        },
    },
    components::{
        badge::{BadgeVariant, badge},
        button::{ButtonSize, button},
        card::{card, card_content, card_description, card_header, card_title},
        field::field_group,
        input::input,
        progress::progress,
        select::select,
        table::{table, table_body, table_cell, table_row},
        textarea::textarea,
    },
    features::{
        auth::require_user,
        issue::{Issue, IssueStatus},
        project::Progress,
        sprint::{Sprint, SprintError, SprintErrors, SprintForm, SprintState},
    },
};

path_param!(sprint_id: u64 else not_found);

/// A rejected sprint, kept to show the submitted values and their errors again.
struct Rejected {
    form: SprintForm,
    errors: SprintErrors,
}

#[page(with(Crumb::new(async |cx| Ok(project(cx).await?.name.clone()))))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let project = project(cx).await?;
    let can_edit = user.role.can_edit();
    let sprints = Sprint::of_project(cx, project.id).await?;
    let issues = Issue::of_project(cx, project.id).await?;
    let rejected = try_context::<Rejected>(cx);

    Ok(view! {
        if rejected.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }

        project_header(project: project, user: user, tab: Tab::Sprints)

        <div class="grid grid-cols-1 gap-6 lg:grid-cols-3 lg:items-start">
            <div class="flex min-w-0 flex-col gap-6 lg:col-span-2">
                planning(sprints: &sprints, issues: &issues, can_edit: can_edit)
            </div>

            if can_edit {
                new_sprint(project_id: project.id, rejected: rejected)
            }
        </div>
    })
}

/// The open sprints, the backlog, and then the completed sprints.
#[component]
async fn planning(
    cx: &Cx,
    sprints: &[Sprint],
    issues: &[Issue],
    can_edit: bool,
) -> Result<impl View> {
    let open = sprints
        .iter()
        .filter(|sprint| sprint.is_open())
        .collect::<Vec<_>>();
    let completed = sprints.iter().filter(|sprint| !sprint.is_open());
    let running = open
        .iter()
        .any(|sprint| sprint.state == SprintState::Active);
    let backlog = issues
        .iter()
        .filter(|issue| issue.sprint_id.is_none() && issue.status != IssueStatus::Done)
        .collect::<Vec<_>>();

    Ok(view! {
        #[key(sprint.id)]
        for &sprint in &open {
            sprint_card(
                sprint: sprint,
                issues: issues,
                choices: &open,
                can_edit: can_edit,
                can_start: !running
            )
        }

        card(
            attrs: attributes! { class="overflow-hidden pb-0" },
            card_header(
                card_title((t!(backlog)))
                card_description((t!(backlog.description, count: backlog.len())))
            )
            issue_rows(
                issues: &backlog,
                choices: &open,
                can_plan: can_edit,
                empty: t!(backlog.empty)
            )
        )

        #[key(sprint.id)]
        for sprint in completed {
            sprint_card(
                sprint: sprint,
                issues: issues,
                choices: &open,
                can_edit: can_edit,
                can_start: false
            )
        }
    })
}

/// A sprint with its dates, progress, actions, and issues. `choices` are the sprints
/// an issue can move to.
#[component]
async fn sprint_card(
    cx: &Cx,
    sprint: &Sprint,
    issues: &[Issue],
    choices: &[&Sprint],
    can_edit: bool,
    can_start: bool,
) -> Result<impl View> {
    let progress_id = element_id(cx);
    let planned = issues
        .iter()
        .filter(|issue| issue.sprint_id == Some(sprint.id))
        .collect::<Vec<_>>();
    let done = Progress::of(planned.iter().map(|issue| issue.status));

    Ok(view! {
        card(
            attrs: attributes! { class="overflow-hidden pb-0" },
            card_header(
                <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
                    card_title((&sprint.name))
                    state_badge(state: sprint.state)
                    if can_edit {
                        <div class="ml-auto flex items-center gap-2">
                            if sprint.state == SprintState::Planned && can_start {
                                <form
                                    method="post"
                                    action=(href!(start, project_id: sprint.project_id, sprint_id: sprint.id))
                                >
                                    button(
                                        size: ButtonSize::Sm,
                                        attrs: attributes! { type="submit" },
                                        (t!(start_sprint))
                                    )
                                </form>
                            }
                            if sprint.state == SprintState::Active {
                                <form
                                    method="post"
                                    action=(href!(
                                        complete,
                                        project_id: sprint.project_id,
                                        sprint_id: sprint.id,
                                    ))
                                >
                                    button(
                                        size: ButtonSize::Sm,
                                        attrs: attributes! { type="submit" },
                                        (t!(complete_sprint))
                                    )
                                </form>
                            }
                            confirm_delete(
                                action: href!(
                                    delete,
                                    project_id: sprint.project_id,
                                    sprint_id: sprint.id,
                                ).resolve(
                                    cx,
                                ),
                                label: t!(delete_sprint, name: &sprint.name),
                                title: t!(confirm_delete_sprint, name: &sprint.name),
                                description: t!(confirm_delete_sprint.description),
                                cancel: t!(keep_sprint),
                                confirm: t!(delete_sprint_confirm),
                                compact: true
                            )
                        </div>
                    }
                </div>
                card_description(
                    date(on: sprint.starts_on)
                    " – "
                    date(on: sprint.ends_on)
                    if !sprint.goal.is_empty() {
                        " · "
                        (&sprint.goal)
                    }
                )
                <div class="mt-2 flex items-center gap-3 text-sm">
                    progress(
                        value: f32::from(done.percent()),
                        attrs: attributes! { aria-labelledby=(&progress_id) }
                    )
                    <span
                        id=(&progress_id)
                        class="shrink-0 text-muted-foreground tabular-nums"
                    >
                        (t!(done_of, done: done.done, total: done.total))
                    </span>
                </div>
            )
            issue_rows(
                issues: &planned,
                choices: choices,
                can_plan: can_edit && sprint.is_open(),
                empty: t!(sprint_empty)
            )
        )
    })
}

/// The issues of a sprint or of the backlog. With `can_plan`, each row has a menu
/// that moves its issue to another sprint or to the backlog.
#[component]
async fn issue_rows(
    cx: &Cx,
    issues: &[&Issue],
    choices: &[&Sprint],
    can_plan: bool,
    empty: Translation<'_>,
) -> Result<impl View> {
    Ok(view! {
        if issues.is_empty() {
            <p class="px-6 pb-6 text-sm text-muted-foreground">(empty)</p>
        } else {
            table(
                table_body(
                    for &issue in issues {
                        table_row(
                            attrs: attributes! { class="relative" },
                            table_cell(issue_key(issue: issue))
                            // The title takes the space the other columns leave and is cut off to fit.
                            table_cell(
                                attrs: attributes! { class="w-full max-w-0 min-w-40" },
                                issue_title(issue: issue)
                            )
                            table_cell(priority_icon(priority: issue.priority))
                            table_cell(status_badge(status: issue.status))
                            if can_plan {
                                // Raised above the row's link so the menu stays usable.
                                table_cell(
                                    attrs: attributes! { class="relative z-10 py-1" },
                                    <form
                                        method="post"
                                        action=(href!(assign, project_id: issue.project_id))
                                    >
                                        <input type="hidden" name="issue" value=(issue.id)>
                                        // Saves as soon as another sprint is picked.
                                        select(
                                            attrs: attributes! {
                                                name="sprint"
                                                class="h-8 w-36"
                                                aria-label=(t!(move_to_sprint))
                                                onchange="this.form.requestSubmit()"
                                            },
                                            <option value="" selected=(issue.sprint_id.is_none())>
                                                (t!(backlog))
                                            </option>
                                            for choice in choices {
                                                <option
                                                    value=(choice.id)
                                                    selected=(issue.sprint_id == Some(choice.id))
                                                >
                                                    (&choice.name)
                                                </option>
                                            }
                                        )
                                    </form>
                                )
                            }
                        )
                    }
                )
            )
        }
    })
}

#[component]
async fn new_sprint(cx: &Cx, project_id: u64, rejected: Option<&Rejected>) -> Result<impl View> {
    let name_id = element_id(cx);
    let goal_id = element_id(cx);
    let start_id = element_id(cx);
    let end_id = element_id(cx);
    let form = rejected.map(|rejected| &rejected.form);
    let errors = rejected.map_or_default(|rejected| rejected.errors);

    Ok(view! {
        card(
            card_header(
                card_title((t!(new_sprint)))
                card_description((t!(new_sprint.description)))
            )
            card_content(
                // Disable browser validation so the form shows the server's field errors.
                <form
                    method="post"
                    action=(href!(plan, project_id: project_id))
                    novalidate=""
                >
                    field_group(
                        labeled(
                            label: t!(field_sprint_name),
                            for_id: &name_id,
                            error: errors.name.map(error_message),
                            input(
                                attrs: attributes! {
                                    id=(&name_id)
                                    name="name"
                                    value=(form.map(|form| form.name.as_str()))
                                    autocomplete="off"
                                    aria-invalid=(errors.name.is_some().then_some("true"))
                                }
                            )
                        )
                        labeled(
                            label: t!(field_goal),
                            for_id: &goal_id,
                            error: errors.goal.map(error_message),
                            textarea(
                                attrs: attributes! {
                                    id=(&goal_id)
                                    name="goal"
                                    aria-invalid=(errors.goal.is_some().then_some("true"))
                                },
                                if let Some(form) = form {
                                    (&form.goal)
                                }
                            )
                        )
                        labeled(
                            label: t!(field_starts_on),
                            for_id: &start_id,
                            error: errors.starts_on.map(error_message),
                            input(
                                attrs: attributes! {
                                    id=(&start_id)
                                    name="starts_on"
                                    type="date"
                                    value=(form.map(|form| form.starts_on.as_str()))
                                    aria-invalid=(errors.starts_on.is_some().then_some("true"))
                                }
                            )
                        )
                        labeled(
                            label: t!(field_ends_on),
                            for_id: &end_id,
                            error: errors.ends_on.map(error_message),
                            input(
                                attrs: attributes! {
                                    id=(&end_id)
                                    name="ends_on"
                                    type="date"
                                    value=(form.map(|form| form.ends_on.as_str()))
                                    aria-invalid=(errors.ends_on.is_some().then_some("true"))
                                }
                            )
                        )
                        button(
                            attrs: attributes! { type="submit" class="self-start" },
                            icon(data: iconify_icon!("lucide:plus"))
                            (t!(plan_sprint))
                        )
                    )
                </form>
            )
        )
    })
}

#[component]
async fn state_badge(cx: &Cx, state: SprintState) -> Result<impl View> {
    let variant = match state {
        SprintState::Active => BadgeVariant::Primary,
        SprintState::Planned => BadgeVariant::Outline,
        SprintState::Completed => BadgeVariant::Secondary,
    };

    Ok(view! { badge(variant: variant, (state_label(state))) })
}

#[component]
async fn date(cx: &Cx, on: Date) -> Result<impl View> {
    Ok(view! {
        <time datetime=(on.to_string())>
            (DateTime::from(on).date_style(DateTimeStyle::Medium))
        </time>
    })
}

pub(super) fn state_label(state: SprintState) -> Translation<'static> {
    match state {
        SprintState::Planned => t!(state_planned),
        SprintState::Active => t!(state_active),
        SprintState::Completed => t!(state_completed),
    }
}

fn error_message(error: SprintError) -> Translation<'static> {
    match error {
        SprintError::NameMissing => t!(error_name_missing),
        SprintError::NameTooLong { max } => t!(error_name_too_long, max),
        SprintError::GoalTooLong { max } => t!(error_description_too_long, max),
        SprintError::DateMissing | SprintError::DateInvalid => t!(error_date_invalid),
        SprintError::EndBeforeStart => t!(error_end_before_start),
    }
}

#[route(POST)]
pub async fn plan(cx: &Cx, Form(form): Form<SprintForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let id = project(cx).await?.id;
    if let Err(errors) = Sprint::plan(cx, id, &form).await? {
        // Rewrite the POST to keep the submitted values and errors in the request context.
        return Err(
            rewrite(href!(page, project_id: id).resolve(cx), Body::empty())
                .method(Method::GET)
                .with(Rejected { form, errors })
                .into(),
        );
    }

    Ok(see_other(href!(page, project_id: id).resolve(cx)))
}

/// Loads the sprint named in the URL, if it belongs to the project in the URL.
async fn sprint(cx: &Cx) -> Result<Sprint> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let project = *project_id(cx)?;
    Ok(Sprint::find(cx, *sprint_id(cx)?)
        .await?
        .filter(|sprint| sprint.project_id == project)
        .ok_or_not_found()?)
}

#[route(POST "./{sprint_id}/start")]
pub async fn start(cx: &Cx) -> Result<SeeOther> {
    let mut sprint = sprint(cx).await?;
    // Another request may have started a sprint since this page was opened.
    // If so, show the sprints unchanged.
    sprint.start(cx).await?;

    Ok(see_other(
        href!(page, project_id: sprint.project_id).resolve(cx),
    ))
}

#[route(POST "./{sprint_id}/complete")]
pub async fn complete(cx: &Cx) -> Result<SeeOther> {
    let mut sprint = sprint(cx).await?;
    sprint.complete(cx).await?;

    Ok(see_other(
        href!(page, project_id: sprint.project_id).resolve(cx),
    ))
}

#[route(POST "./{sprint_id}/delete")]
pub async fn delete(cx: &Cx) -> Result<SeeOther> {
    let sprint = sprint(cx).await?;
    let project = sprint.project_id;
    sprint.remove(cx).await?;

    Ok(see_other(href!(page, project_id: project).resolve(cx)))
}

#[derive(Deserialize)]
pub struct Assign {
    issue: u64,
    /// The sprint's ID. Blank returns the issue to the backlog.
    sprint: String,
}

#[route(POST "./assign")]
pub async fn assign(cx: &Cx, Form(form): Form<Assign>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let id = *project_id(cx)?;
    let mut issue = Issue::find(cx, form.issue)
        .await?
        .filter(|issue| issue.project_id == id)
        .ok_or_not_found()?;
    let sprint = match form.sprint.trim() {
        "" => None,
        sprint => Some(sprint.parse::<u64>().ok().ok_or_not_found()?),
    };
    // A sprint that was completed or deleted since the page was opened is ignored.
    issue.set_sprint(cx, sprint).await?;

    Ok(see_other(href!(page, project_id: id).resolve(cx)))
}
