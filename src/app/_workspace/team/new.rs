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

use super::{t, user_fields, user_id};
use crate::{
    app::_workspace::Crumb,
    components::{
        button::{ButtonSize, ButtonVariant, button, button_variants},
        card::{card, card_content, card_footer},
    },
    features::{
        auth::require_user,
        user::{User, UserErrors, UserForm},
    },
};

struct Rejected {
    form: UserForm,
    errors: UserErrors,
}

#[page(with(Crumb::new(async |cx| Ok(t!(add_member).resolve(cx).into_owned()))))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    user.role.can_manage().then_some(()).ok_or_forbidden()?;

    let rejected = try_context::<Rejected>(cx);
    let form = rejected.map_or_else(UserForm::new, |rejected| rejected.form.clone());
    let errors = rejected.map_or_default(|rejected| rejected.errors);
    let form_id = element_id(cx);

    Ok(view! {
        if rejected.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }

        <header class="flex flex-col gap-1">
            <h1 class="text-2xl font-semibold tracking-tight">(t!(add_member))</h1>
            <p class="text-muted-foreground">(t!(add_member.description))</p>
        </header>

        card(
            attrs: attributes! { class="max-w-2xl" },
            card_content(
                // Disable browser validation so the form shows the server's field errors.
                <form id=(&form_id) method="post" action=(href!(create)) novalidate="">
                    user_fields(
                        form: &form,
                        errors: errors,
                        password_hint: t!(field_password.new)
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
                    (t!(add_member))
                )
            )
        )
    })
}

#[route(POST)]
pub async fn create(cx: &Cx, Form(form): Form<UserForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_manage().then_some(()).ok_or_forbidden()?;

    match User::invite(cx, &form).await? {
        Ok(member) => Ok(see_other(
            href!(user_id::page, user_id: member.id).resolve(cx),
        )),
        // Rewrite the POST to keep the submitted values and errors in the request context.
        Err(errors) => Err(rewrite(href!(page).resolve(cx), Body::empty())
            .method(Method::GET)
            .with(Rejected { form, errors })
            .into()),
    }
}
