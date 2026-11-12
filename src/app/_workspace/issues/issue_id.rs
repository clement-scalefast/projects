use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, memoize, try_context},
    icon::{icon, iconify::iconify_icon},
    l10n::Translation,
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{RouterErrorExt, SeeOther, rewrite, see_other},
        href, module_param, page, route,
    },
    runtime::{Event, link, signal},
    view::{Child, View, attributes, component, element_id, view},
};

use super::{
    assignee, due_date, issue_fields, kind_icon, kind_label, priority_badge, status_badge,
    status_label, t,
};
use crate::{
    app::{
        _common::time_ago,
        _workspace::{Crumb, projects::project_id, team::user_id},
    },
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        button::{ButtonSize, ButtonVariant, button, button_variants},
        card::{card, card_content, card_description, card_footer, card_header, card_title},
        dialog::{
            dialog, dialog_content, dialog_description, dialog_footer, dialog_header, dialog_title,
        },
        field::field_error,
        select::select,
        textarea::textarea,
    },
    features::{
        auth::require_user,
        issue::{Comment, CommentError, Issue, IssueErrors, IssueForm, IssueStatus},
        stats::today,
        user::User,
    },
};

module_param!(pub issue_id: u64 else not_found);

/// A rejected edit, kept to show the submitted values and their errors again.
struct Rejected {
    form: IssueForm,
    errors: IssueErrors,
}

/// A rejected comment, kept to show the submitted text and its error again.
struct RejectedComment {
    body: String,
    error: CommentError,
}

#[memoize(as_ref)]
async fn issue(cx: &Cx) -> Result<Issue> {
    Ok(Issue::find(cx, *issue_id(cx)?).await?.ok_or_not_found()?)
}

#[page(with(Crumb::new(async |cx| Ok(issue(cx).await?.key()))))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let issue = issue(cx).await?;
    let can_edit = user.role.can_edit();
    let rejected = try_context::<RejectedComment>(cx);

    Ok(view! {
        if rejected.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }

        <header class="flex flex-col gap-4 sm:flex-row sm:items-start">
            <div class="flex min-w-0 flex-col gap-2">
                <p class="flex items-center gap-2 text-sm text-muted-foreground">
                    kind_icon(kind: issue.kind)
                    <span class="font-mono">(issue.key())</span>
                    <span aria-hidden="true">"·"</span>
                    (kind_label(issue.kind))
                </p>
                <h1 class="text-2xl font-semibold tracking-tight text-balance">
                    (&issue.title)
                </h1>
            </div>
            if can_edit {
                <div class="flex shrink-0 gap-2 sm:ml-auto">
                    link(
                        href: href!(edit, issue_id: issue.id),
                        attrs: attributes! {
                            class=(button_variants(
                                ButtonVariant::Outline,
                                ButtonSize::Md,
                            ))
                        },
                        icon(data: iconify_icon!("lucide:pencil"))
                        (t!(edit))
                    )
                    delete_button(issue: issue)
                </div>
            }
        </header>

        <div class="grid grid-cols-1 gap-6 lg:grid-cols-3 lg:items-start">
            <div class="flex flex-col gap-6 lg:col-span-2">
                card(
                    card_header(card_title((t!(description))))
                    card_content(
                        if issue.description.is_empty() {
                            <p class="text-sm text-muted-foreground">
                                (t!(no_description))
                            </p>
                        } else {
                            <p class="text-sm leading-relaxed whitespace-pre-wrap">
                                (&issue.description)
                            </p>
                        }
                    )
                )
                comments(issue: issue, user: user, rejected: rejected)
            </div>

            details(issue: issue, can_edit: can_edit)
        </div>
    })
}

/// The side panel with the issue's status, people, and dates.
#[component]
async fn details(cx: &Cx, issue: &Issue, can_edit: bool) -> Result<impl View> {
    let status_id = element_id(cx);
    let project = issue.project.get();
    let reporter = issue.reporter.get();

    Ok(view! {
        card(
            card_header(card_title((t!(details))))
            card_content(
                <dl class="flex flex-col gap-4 text-sm">
                    detail(
                        label: t!(field_status),
                        if can_edit {
                            <form
                                method="post"
                                action=(href!(set_status, issue_id: issue.id))
                            >
                                <label for=(&status_id) class="sr-only">
                                    (t!(field_status))
                                </label>
                                // Saves as soon as another status is picked.
                                select(
                                    attrs: attributes! {
                                        id=(&status_id)
                                        name="status"
                                        onchange="this.form.requestSubmit()"
                                    },
                                    for status in IssueStatus::ALL {
                                        <option
                                            value=(status.slug())
                                            selected=(issue.status == status)
                                        >
                                            (status_label(status))
                                        </option>
                                    }
                                )
                            </form>
                        } else {
                            status_badge(status: issue.status)
                        }
                    )
                    detail(
                        label: t!(field_priority),
                        priority_badge(priority: issue.priority)
                    )
                    detail(
                        label: t!(field_assignee),
                        if let Some(user) = issue.assignee.get() {
                            person(user: user)
                        } else {
                            assignee(user: None)
                        }
                    )
                    detail(label: t!(reporter), person(user: reporter))
                    detail(
                        label: t!(field_project),
                        link(
                            href: href!(project_id::page, project_id: project.id),
                            attrs: attributes! { class="font-medium hover:underline" },
                            (&project.name)
                        )
                    )
                    detail(
                        label: t!(field_due),
                        if let Some(due) = issue.due_on {
                            due_date(
                                due: due,
                                overdue: issue.is_overdue(today(jiff::Timestamp::now()))
                            )
                        } else {
                            <span class="text-muted-foreground">(t!(no_due_date))</span>
                        }
                    )
                    detail(label: t!(created), time_ago(at: issue.created_at))
                    detail(label: t!(updated), time_ago(at: issue.updated_at))
                </dl>
            )
        )
    })
}

/// A row of the details panel.
#[component]
async fn detail(cx: &Cx, label: Translation<'_>, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div class="grid grid-cols-[6.5rem_1fr] items-center gap-2">
            <dt class="text-muted-foreground">(label)</dt>
            <dd class="min-w-0">(child)</dd>
        </div>
    })
}

/// A team member's avatar and name, linked to their page.
#[component]
async fn person(cx: &Cx, user: &User) -> Result<impl View> {
    Ok(view! {
        link(
            href: href!(user_id::page, user_id: user.id),
            attrs: attributes! { class="flex items-center gap-2 hover:underline" },
            avatar(size: AvatarSize::Sm, avatar_fallback((user.initials())))
            <span class="truncate">(&user.name)</span>
        )
    })
}

#[component]
async fn comments(
    cx: &Cx,
    issue: &Issue,
    user: &User,
    rejected: Option<&RejectedComment>,
) -> Result<impl View> {
    let comments = Comment::of_issue(cx, issue.id).await?;
    let body_id = element_id(cx);
    let error_id = element_id(cx);

    Ok(view! {
        card(
            card_header(
                card_title((t!(comments)))
                card_description((t!(comment_count, count: comments.len())))
            )
            card_content(
                attrs: attributes! { class="flex flex-col gap-6" },
                if !comments.is_empty() {
                    <ol class="flex flex-col gap-5">
                        for comment in &comments {
                            let author = comment.author.get();
                            <li class="flex gap-3">
                                avatar(
                                    size: AvatarSize::Sm,
                                    avatar_fallback((author.initials()))
                                )
                                <div class="flex min-w-0 flex-1 flex-col gap-1 text-sm">
                                    <p class="flex flex-wrap items-baseline gap-x-2">
                                        <span class="font-medium">(&author.name)</span>
                                        <span class="text-xs text-muted-foreground">
                                            time_ago(at: comment.created_at)
                                        </span>
                                    </p>
                                    <p class="leading-relaxed whitespace-pre-wrap">
                                        (&comment.body)
                                    </p>
                                </div>
                            </li>
                        }
                    </ol>
                }
                if user.role.can_edit() {
                    <form
                        method="post"
                        action=(href!(add_comment, issue_id: issue.id))
                        class="flex flex-col gap-3"
                        novalidate=""
                    >
                        <label for=(&body_id) class="sr-only">(t!(add_comment))</label>
                        textarea(
                            attrs: attributes! {
                                id=(&body_id)
                                name="body"
                                placeholder=(t!(add_comment.placeholder))
                                aria-invalid=(rejected.is_some().then_some("true"))
                                aria-describedby=(rejected.is_some().then_some(&error_id))
                            },
                            if let Some(rejected) = rejected {
                                (&rejected.body)
                            }
                        )
                        if let Some(rejected) = rejected {
                            field_error(
                                attrs: attributes! { id=(&error_id) },
                                (comment_error(rejected.error))
                            )
                        }
                        button(
                            attrs: attributes! { type="submit" class="self-end" },
                            (t!(add_comment))
                        )
                    </form>
                }
            )
        )
    })
}

fn comment_error(error: CommentError) -> Translation<'static> {
    match error {
        CommentError::Missing => t!(error_comment_missing),
        CommentError::TooLong { max } => t!(error_comment_too_long, max),
    }
}

/// A delete button that asks for confirmation in a dialog.
#[component]
async fn delete_button(cx: &Cx, issue: &Issue) -> Result<impl View> {
    let confirming = signal(cx, || false);
    let dialog_id = element_id(cx);
    let title_id = element_id(cx);

    Ok(view! {
        button(
            variant: ButtonVariant::Outline,
            attrs: attributes! {
                type="button"
                aria-haspopup="dialog"
                aria-controls=(&dialog_id)
                @click=$(|_e: Event| confirming.set(true))
            },
            icon(data: iconify_icon!("lucide:trash-2"))
            (t!(delete))
        )

        dialog(
            open: $(confirming.get()),
            attrs: attributes! {
                id=(&dialog_id)
                aria-labelledby=(&title_id)
                @keydown=$(|e: Event| {
                    if e.key == "Escape" {
                        confirming.set(false);
                    }
                })
                @click=$(|e: Event| {
                    // Backdrop clicks target the dialog; clicks inside it target its children.
                    if e.target.id == dialog_id {
                        confirming.set(false);
                    }
                })
            },
            dialog_content(
                dialog_header(
                    dialog_title(
                        attrs: attributes! { id=(&title_id) },
                        (t!(confirm_delete, key: issue.key()))
                    )
                    dialog_description((t!(confirm_delete.description)))
                )
                dialog_footer(
                    button(
                        variant: ButtonVariant::Outline,
                        attrs: attributes! {
                            type="button"
                            @click=$(|_e: Event| confirming.set(false))
                        },
                        (t!(keep_issue))
                    )
                    <form method="post" action=(href!(delete, issue_id: issue.id))>
                        button(
                            variant: ButtonVariant::Destructive,
                            attrs: attributes! { type="submit" },
                            (t!(delete_issue))
                        )
                    </form>
                )
            )
        )
    })
}

#[page("./edit", with(Crumb::new(async |cx| Ok(issue(cx).await?.key()))))]
pub async fn edit(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;
    let issue = issue(cx).await?;

    let rejected = try_context::<Rejected>(cx);
    let form = rejected.map_or_else(|| IssueForm::of(issue), |rejected| rejected.form.clone());
    let errors = rejected.map(|rejected| rejected.errors).unwrap_or_default();
    let users = User::list(cx).await?;
    let form_id = element_id(cx);

    Ok(view! {
        if rejected.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }

        <header class="flex flex-col gap-1">
            <h1 class="text-2xl font-semibold tracking-tight">
                (t!(edit_issue, key: issue.key()))
            </h1>
            <p class="text-muted-foreground">(&issue.project.get().name)</p>
        </header>

        card(
            attrs: attributes! { class="max-w-3xl" },
            card_content(
                // Disable browser validation so the form shows the server's field errors.
                <form
                    id=(&form_id)
                    method="post"
                    action=(href!(save, issue_id: issue.id))
                    novalidate=""
                >
                    issue_fields(
                        form: &form,
                        errors: errors,
                        projects: None,
                        users: &users
                    )
                </form>
            )
            card_footer(
                attrs: attributes! { class="justify-end" },
                link(
                    href: href!(page, issue_id: issue.id),
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

#[route(POST "./edit")]
pub async fn save(cx: &Cx, Form(form): Form<IssueForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let id = *issue_id(cx)?;
    let mut issue = Issue::find(cx, id).await?.ok_or_not_found()?;
    if let Err(errors) = issue.edit(cx, &form).await? {
        // Rewrite the POST to keep the submitted values and errors in the request context.
        return Err(
            rewrite(href!(edit, issue_id: id).resolve(cx), Body::empty())
                .method(Method::GET)
                .with(Rejected { form, errors })
                .into(),
        );
    }

    Ok(see_other(href!(page, issue_id: id).resolve(cx)))
}

#[derive(Deserialize)]
pub struct SetStatus {
    status: IssueStatus,
}

#[route(POST "./status")]
pub async fn set_status(cx: &Cx, Form(form): Form<SetStatus>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let id = *issue_id(cx)?;
    let mut issue = Issue::find(cx, id).await?.ok_or_not_found()?;
    issue.move_to(cx, form.status).await?;

    Ok(see_other(href!(page, issue_id: id).resolve(cx)))
}

#[derive(Deserialize)]
pub struct NewComment {
    body: String,
}

#[route(POST "./comments")]
pub async fn add_comment(cx: &Cx, Form(form): Form<NewComment>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let id = *issue_id(cx)?;
    let mut issue = Issue::find(cx, id).await?.ok_or_not_found()?;
    if let Err(error) = Comment::add(cx, &mut issue, user, &form.body).await? {
        // Rewrite the POST to keep the submitted text and error in the request context.
        return Err(
            rewrite(href!(page, issue_id: id).resolve(cx), Body::empty())
                .method(Method::GET)
                .with(RejectedComment {
                    body: form.body,
                    error,
                })
                .into(),
        );
    }

    Ok(see_other(href!(page, issue_id: id).resolve(cx)))
}

#[route(POST "./delete")]
pub async fn delete(cx: &Cx) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let issue = Issue::find(cx, *issue_id(cx)?).await?.ok_or_not_found()?;
    let project = issue.project_id;
    issue.remove(cx).await?;

    Ok(see_other(
        href!(project_id::page, project_id: project).resolve(cx),
    ))
}
