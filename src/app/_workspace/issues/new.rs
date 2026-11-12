use serde::Serialize;
use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{RouterErrorExt, SeeOther, rewrite, see_other},
        href, page, query_params, route,
    },
    runtime::link,
    view::{View, attributes, element_id, view},
};

use super::{issue_fields, issue_id, t};
use crate::{
    app::_workspace::Crumb,
    components::{
        button::{ButtonSize, ButtonVariant, button, button_variants},
        card::{card, card_content, card_footer},
    },
    features::{
        auth::require_user,
        issue::{Issue, IssueErrors, IssueForm},
        project::Project,
        user::User,
    },
};

/// Preselects a project, such as when the form is opened from a project's board.
#[query_params(error = redirect("?"))]
#[derive(Serialize)]
pub struct NewIssueQuery {
    project: Option<u64>,
}

impl NewIssueQuery {
    pub fn in_project(id: u64) -> Self {
        Self { project: Some(id) }
    }
}

struct Rejected {
    form: IssueForm,
    errors: IssueErrors,
}

#[page(with(Crumb::new(async |cx| Ok(t!(new_issue).resolve(cx).into_owned()))))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let rejected = try_context::<Rejected>(cx);
    let form = match rejected {
        Some(rejected) => rejected.form.clone(),
        None => IssueForm::new(query_params::<NewIssueQuery>(cx)?.project),
    };
    let errors = rejected.map(|rejected| rejected.errors).unwrap_or_default();
    let projects = Project::names(cx).await?;
    let users = User::list(cx).await?;
    let form_id = element_id(cx);

    Ok(view! {
        if rejected.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }

        <header class="flex flex-col gap-1">
            <h1 class="text-2xl font-semibold tracking-tight">(t!(new_issue))</h1>
            <p class="text-muted-foreground">(t!(new_issue.description))</p>
        </header>

        card(
            attrs: attributes! { class="max-w-3xl" },
            card_content(
                // Disable browser validation so the form shows the server's field errors.
                <form id=(&form_id) method="post" action=(href!(create)) novalidate="">
                    issue_fields(
                        form: &form,
                        errors: errors,
                        projects: Some(projects.as_slice()),
                        users: &users
                    )
                </form>
            )
            card_footer(
                attrs: attributes! { class="justify-end" },
                link(
                    href: href!(super::page),
                    attrs: attributes! {
                        class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
                    },
                    (t!(cancel))
                )
                button(
                    attrs: attributes! { type="submit" form=(&form_id) },
                    (t!(create_issue))
                )
            )
        )
    })
}

#[route(POST)]
pub async fn create(cx: &Cx, Form(form): Form<IssueForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    match Issue::open(cx, user, &form).await? {
        Ok(issue) => Ok(see_other(
            href!(issue_id::page, issue_id: issue.id).resolve(cx),
        )),
        // Rewrite the POST to keep the submitted values and errors in the request context.
        Err(errors) => Err(rewrite(href!(page).resolve(cx), Body::empty())
            .method(Method::GET)
            .with(Rejected { form, errors })
            .into()),
    }
}
