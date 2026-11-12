use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{RouterErrorExt, SeeOther, rewrite, see_other},
        href, page, route,
    },
    runtime::link,
    view::{View, attributes, element_id, view},
};

use super::{project_fields, project_id, t};
use crate::{
    app::_workspace::Crumb,
    components::{
        button::{ButtonSize, ButtonVariant, button, button_variants},
        card::{card, card_content, card_footer},
    },
    features::{
        auth::require_user,
        project::{Project, ProjectErrors, ProjectForm},
        user::User,
    },
};

struct Rejected {
    form: ProjectForm,
    errors: ProjectErrors,
}

#[page(with(Crumb::new(async |cx| Ok(t!(new_project).resolve(cx).into_owned()))))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    user.role.can_manage().then_some(()).ok_or_forbidden()?;

    let rejected = try_context::<Rejected>(cx);
    let form = rejected.map_or_else(
        || ProjectForm {
            lead: user.id.to_string(),
            ..ProjectForm::default()
        },
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
            <h1 class="text-2xl font-semibold tracking-tight">(t!(new_project))</h1>
            <p class="text-muted-foreground">(t!(new_project.description))</p>
        </header>

        card(
            attrs: attributes! { class="max-w-2xl" },
            card_content(
                // Disable browser validation so the form shows the server's field errors.
                <form id=(&form_id) method="post" action=(href!(create)) novalidate="">
                    project_fields(form: &form, errors: errors, users: &users)
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
                    (t!(create_project))
                )
            )
        )
    })
}

#[route(POST)]
pub async fn create(cx: &Cx, Form(form): Form<ProjectForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_manage().then_some(()).ok_or_forbidden()?;

    match Project::start(cx, &form).await? {
        Ok(project) => Ok(see_other(
            href!(project_id::page, project_id: project.id).resolve(cx),
        )),
        // Rewrite the POST to keep the submitted values and errors in the request context.
        Err(errors) => Err(rewrite(href!(page).resolve(cx), Body::empty())
            .method(Method::GET)
            .with(Rejected { form, errors })
            .into()),
    }
}
