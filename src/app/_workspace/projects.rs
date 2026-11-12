pub(super) mod new;
pub(super) mod project_id;

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    l10n::{Translation, fluent::messages},
    router::{Slot, href, layout, page},
    runtime::link,
    view::{View, attributes, class, component, element_id, view},
};

use super::{Section, issues::labeled};
use crate::{
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        badge::{BadgeVariant, badge},
        button::{ButtonSize, ButtonVariant, button_variants},
        card::{card, card_content, card_description, card_footer, card_header, card_title},
        field::field_group,
        input::input,
        progress::progress,
        select::select,
        textarea::textarea,
    },
    features::{
        auth::require_user,
        project::{Project, ProjectError, ProjectErrors, ProjectForm},
        user::User,
    },
};

messages!("{mod}/l10n/{locale}.ftl");

/// Marks the project pages as part of the projects section.
#[layout(with(Section::Projects))]
async fn section(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { (slot) })
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let projects = Project::list(cx).await?;

    Ok(view! {
        <header class="flex flex-wrap items-center gap-4">
            <div class="flex flex-col gap-1">
                <h1 class="text-2xl font-semibold tracking-tight">(t!(title))</h1>
                <p class="text-muted-foreground">(t!(title.description))</p>
            </div>
            if user.role.can_manage() {
                link(
                    href: href!(new::page),
                    attrs: attributes! {
                        class=(class!(
                            button_variants(ButtonVariant::Primary, ButtonSize::Md),
                            "ml-auto",
                        ))
                    },
                    icon(data: iconify_icon!("lucide:plus"))
                    (t!(new_project))
                )
            }
        </header>

        if projects.is_empty() {
            card(
                card_content(
                    attrs: attributes! { class="py-10 text-center text-sm text-muted-foreground" },
                    (t!(no_projects))
                )
            )
        } else {
            <div class="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
                #[key(project.id)]
                for project in &projects {
                    project_card(project: project)
                }
            </div>
        }
    })
}

/// A summary of a project that links to its board. The lead and issues must
/// already be loaded.
#[component]
async fn project_card(cx: &Cx, project: &Project) -> Result<impl View> {
    let progress_id = element_id(cx);
    let lead = project.lead.get();
    let done = project.progress();

    Ok(view! {
        card(
            attrs: attributes! {
                class="relative transition-colors hover:border-foreground/25"
            },
            card_header(
                <div class="flex items-center gap-2">
                    project_code(code: &project.code)
                    card_title(
                        link(
                            href: href!(project_id::page, project_id: project.id),
                            attrs: attributes! { class="after:absolute after:inset-0" },
                            (&project.name)
                        )
                    )
                </div>
                card_description(
                    attrs: attributes! { class="line-clamp-2 min-h-10" },
                    (&project.description)
                )
            )
            card_content(
                attrs: attributes! { class="flex flex-col gap-2" },
                <div class="flex items-baseline justify-between gap-2 text-sm">
                    <span id=(&progress_id) class="text-muted-foreground">
                        (t!(done_of, done: done.done, total: done.total))
                    </span>
                    <span class="font-medium tabular-nums">
                        (done.percent())
                        "%"
                    </span>
                </div>
                progress(
                    value: f32::from(done.percent()),
                    attrs: attributes! { aria-labelledby=(&progress_id) }
                )
            )
            card_footer(
                attrs: attributes! { class="justify-between text-sm text-muted-foreground" },
                <span class="flex min-w-0 items-center gap-2">
                    avatar(size: AvatarSize::Sm, avatar_fallback((lead.initials())))
                    <span class="truncate">(&lead.name)</span>
                </span>
                <span class="shrink-0">(t!(open_count, count: done.open()))</span>
            )
        )
    })
}

/// The project key as a small badge.
#[component]
pub(super) async fn project_code(cx: &Cx, code: &str) -> Result<impl View> {
    Ok(view! {
        badge(
            variant: BadgeVariant::Secondary,
            attrs: attributes! { class="font-mono" },
            (code)
        )
    })
}

/// The fields for creating or editing a project, to place inside a `<form>`.
#[component]
async fn project_fields(
    cx: &Cx,
    form: &ProjectForm,
    errors: ProjectErrors,
    users: &[User],
) -> Result<impl View> {
    let name_id = element_id(cx);
    let code_id = element_id(cx);
    let description_id = element_id(cx);
    let lead_id = element_id(cx);

    Ok(view! {
        field_group(
            <div class="grid grid-cols-1 gap-6 sm:grid-cols-3">
                <div class="sm:col-span-2">
                    labeled(
                        label: t!(field_name),
                        for_id: &name_id,
                        error: errors.name.map(error_message),
                        input(
                            attrs: attributes! {
                                id=(&name_id)
                                name="name"
                                value=(&form.name)
                                autocomplete="off"
                                aria-invalid=(errors.name.is_some().then_some("true"))
                            }
                        )
                    )
                </div>
                labeled(
                    label: t!(field_code),
                    for_id: &code_id,
                    error: errors.code.map(error_message),
                    input(
                        attrs: attributes! {
                            id=(&code_id)
                            name="code"
                            value=(&form.code)
                            autocomplete="off"
                            placeholder="WEB"
                            class="font-mono uppercase"
                            aria-invalid=(errors.code.is_some().then_some("true"))
                        }
                    )
                )
            </div>
            labeled(
                label: t!(field_description),
                for_id: &description_id,
                error: errors.description.map(error_message),
                textarea(
                    attrs: attributes! {
                        id=(&description_id)
                        name="description"
                        aria-invalid=(errors.description.is_some().then_some("true"))
                    },
                    (&form.description)
                )
            )
            labeled(
                label: t!(field_lead),
                for_id: &lead_id,
                error: errors.lead.map(error_message),
                select(
                    attrs: attributes! {
                        id=(&lead_id)
                        name="lead"
                        aria-invalid=(errors.lead.is_some().then_some("true"))
                    },
                    <option value="" selected=(form.lead.is_empty())>
                        (t!(field_lead.choose))
                    </option>
                    for user in users {
                        <option
                            value=(user.id)
                            selected=(form.lead == user.id.to_string())
                        >
                            (&user.name)
                        </option>
                    }
                )
            )
        )
    })
}

fn error_message(error: ProjectError) -> Translation<'static> {
    match error {
        ProjectError::NameMissing => t!(error_name_missing),
        ProjectError::NameTooLong { max } => t!(error_name_too_long, max),
        ProjectError::CodeMissing => t!(error_code_missing),
        ProjectError::CodeInvalid { min, max } => t!(error_code_invalid, min, max),
        ProjectError::CodeTaken => t!(error_code_taken),
        ProjectError::DescriptionTooLong { max } => t!(error_description_too_long, max),
        ProjectError::LeadMissing => t!(error_lead_missing),
    }
}
