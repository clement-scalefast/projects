use topcoat::{
    Result,
    context::{Cx, try_context},
    l10n::{Translation, fluent::messages},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{RouterErrorExt, SeeOther, rewrite, see_other},
        href, page, path_param, route,
    },
    view::{StaticClass, View, attributes, class, component, element_id, view},
};

use super::{Section, issues::labeled};
use crate::{
    app::_common::confirm_delete,
    components::{
        badge::{BadgeVariant, badge},
        button::button,
        card::{card, card_content, card_description, card_header, card_title},
        field::field_group,
        input::input,
        select::select,
        table::{table, table_body, table_cell, table_head, table_header, table_row},
    },
    features::{
        auth::require_user,
        label::{IssueLabel, Label, LabelColor, LabelError, LabelForm},
    },
};

messages!("{mod}/l10n/{locale}.ftl");

path_param!(label_id: u64 else not_found);

/// A rejected label, kept to show the submitted values and the error again.
struct Rejected {
    form: LabelForm,
    error: LabelError,
}

#[page(with(Section::Labels))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let can_edit = user.role.can_edit();
    let labels = Label::list(cx).await?;
    let usage = IssueLabel::usage(cx).await?;
    let rejected = try_context::<Rejected>(cx);

    Ok(view! {
        if rejected.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }

        <header class="flex flex-col gap-1">
            <h1 class="text-2xl font-semibold tracking-tight">(t!(title))</h1>
            <p class="text-muted-foreground">(t!(title.description))</p>
        </header>

        <div class="grid grid-cols-1 gap-6 lg:grid-cols-3 lg:items-start">
            card(
                attrs: attributes! { class="overflow-hidden py-0 lg:col-span-2" },
                if labels.is_empty() {
                    <p class="p-6 text-sm text-muted-foreground">(t!(no_labels))</p>
                } else {
                    table(
                        table_header(
                            table_row(
                                table_head(
                                    attrs: attributes! { class="w-full" },
                                    (t!(column_label))
                                )
                                table_head(
                                    attrs: attributes! { class="text-right" },
                                    (t!(column_issues))
                                )
                                if can_edit {
                                    table_head(
                                        <span class="sr-only">(t!(column_actions))</span>
                                    )
                                }
                            )
                        )
                        table_body(
                            #[key(label.id)]
                            for label in &labels {
                                table_row(
                                    table_cell(label_badge(label: label))
                                    table_cell(
                                        attrs: attributes! { class="text-right text-muted-foreground tabular-nums" },
                                        (usage.get(&label.id).copied().unwrap_or(0))
                                    )
                                    if can_edit {
                                        table_cell(
                                            attrs: attributes! { class="py-1 text-right" },
                                            confirm_delete(
                                                action: href!(delete, label_id: label.id).resolve(cx),
                                                label: t!(delete, name: &label.name),
                                                title: t!(confirm_delete, name: &label.name),
                                                description: t!(confirm_delete.description),
                                                cancel: t!(keep_label),
                                                confirm: t!(delete_label),
                                                compact: true
                                            )
                                        )
                                    }
                                )
                            }
                        )
                    )
                }
            )

            if can_edit {
                new_label(rejected: rejected)
            }
        </div>
    })
}

#[component]
async fn new_label(cx: &Cx, rejected: Option<&Rejected>) -> Result<impl View> {
    let name_id = element_id(cx);
    let color_id = element_id(cx);
    let name = rejected.map(|rejected| rejected.form.name.as_str());
    let color = rejected.map_or(LabelColor::Gray, |rejected| rejected.form.color);

    Ok(view! {
        card(
            card_header(
                card_title((t!(new_label)))
                card_description((t!(new_label.description)))
            )
            card_content(
                // Disable browser validation so the form shows the server's field errors.
                <form method="post" action=(href!(create)) novalidate="">
                    field_group(
                        labeled(
                            label: t!(field_name),
                            for_id: &name_id,
                            error: rejected.map(|rejected| error_message(rejected.error)),
                            input(
                                attrs: attributes! {
                                    id=(&name_id)
                                    name="name"
                                    value=(name)
                                    autocomplete="off"
                                    aria-invalid=(rejected.is_some().then_some("true"))
                                }
                            )
                        )
                        labeled(
                            label: t!(field_color),
                            for_id: &color_id,
                            error: None,
                            select(
                                attrs: attributes! { id=(&color_id) name="color" },
                                for option in LabelColor::ALL {
                                    <option value=(option.slug()) selected=(option == color)>
                                        (color_label(option))
                                    </option>
                                }
                            )
                        )
                        button(
                            attrs: attributes! { type="submit" class="self-start" },
                            (t!(add_label))
                        )
                    )
                </form>
            )
        )
    })
}

/// A label's name with a dot in its color.
#[component]
pub(super) async fn label_badge(cx: &Cx, label: &Label) -> Result<impl View> {
    let dot: StaticClass = match label.color {
        LabelColor::Gray => class!("bg-muted-foreground/60"),
        LabelColor::Red => class!("bg-rose-500"),
        LabelColor::Amber => class!("bg-amber-500"),
        LabelColor::Green => class!("bg-emerald-500"),
        LabelColor::Sky => class!("bg-sky-500"),
        LabelColor::Violet => class!("bg-violet-500"),
        LabelColor::Pink => class!("bg-pink-500"),
    };

    Ok(view! {
        badge(
            variant: BadgeVariant::Secondary,
            attrs: attributes! { class="whitespace-nowrap" },
            <span class=(class!("size-1.5 shrink-0 rounded-full", dot))></span>
            (&label.name)
        )
    })
}

fn color_label(color: LabelColor) -> Translation<'static> {
    match color {
        LabelColor::Gray => t!(color_gray),
        LabelColor::Red => t!(color_red),
        LabelColor::Amber => t!(color_amber),
        LabelColor::Green => t!(color_green),
        LabelColor::Sky => t!(color_sky),
        LabelColor::Violet => t!(color_violet),
        LabelColor::Pink => t!(color_pink),
    }
}

fn error_message(error: LabelError) -> Translation<'static> {
    match error {
        LabelError::Missing => t!(error_name_missing),
        LabelError::TooLong { max } => t!(error_name_too_long, max),
        LabelError::Taken => t!(error_name_taken),
    }
}

#[route(POST)]
pub async fn create(cx: &Cx, Form(form): Form<LabelForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    if let Err(error) = Label::add(cx, &form).await? {
        // Rewrite the POST to keep the submitted values and error in the request context.
        return Err(rewrite(href!(page).resolve(cx), Body::empty())
            .method(Method::GET)
            .with(Rejected { form, error })
            .into());
    }

    Ok(see_other(href!(page).resolve(cx)))
}

#[route(POST "./{label_id}/delete")]
pub async fn delete(cx: &Cx) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let label = Label::find(cx, *label_id(cx)?).await?.ok_or_not_found()?;
    label.remove(cx).await?;

    Ok(see_other(href!(page).resolve(cx)))
}
