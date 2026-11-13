mod callback;
mod start;

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    l10n::fluent::messages,
    router::{error::redirect, href, page, query_params},
    view::{View, attributes, view},
};

use super::{_common::role_label, language::language_menu};
use crate::{
    components::{
        badge::{BadgeVariant, badge},
        button::{ButtonSize, ButtonVariant, button_variants},
        card::{card, card_content, card_description, card_footer, card_header, card_title},
    },
    features::auth::current_user,
    seed::{DEMO_PASSWORD, DEMO_USERS},
};

messages!("{mod}/l10n/{locale}.ftl");

#[query_params(error = redirect("?"))]
struct LoginQuery {
    /// Present when a sign-in at Keycloak did not complete.
    failed: Option<String>,
}

/// Explains that Keycloak handles sign-in and links to it. Signing out returns here.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    if current_user(cx).await?.is_some() {
        return Err(redirect(href!(super::_workspace::_dashboard::page).resolve(cx)).into());
    }
    let failed = query_params::<LoginQuery>(cx)?.failed.is_some();

    Ok(view! {
        <main
            class="relative flex min-h-svh flex-col items-center justify-center gap-6 p-6"
        >
            <div class="absolute top-3 right-3">language_menu()</div>

            <p class="flex items-center gap-2 text-lg font-semibold">
                icon(data: iconify_icon!("lucide:square-kanban"))
                "Projects"
            </p>

            card(
                attrs: attributes! { class="w-full max-w-sm" },
                card_header(
                    card_title((t!(sign_in)))
                    card_description((t!(sign_in.description)))
                )
                if failed {
                    card_content(
                        <p role="alert" class="text-sm text-destructive">
                            (t!(error_sign_in_failed))
                        </p>
                    )
                }
                card_footer(
                    <a
                        href=(href!(start::start))
                        class=((
                            button_variants(ButtonVariant::Primary, ButtonSize::Md),
                            "w-full",
                        ))
                    >
                        (t!(sign_in.with_keycloak))
                    </a>
                )
            )

            demo_accounts()
        </main>
    })
}

#[topcoat::view::component]
async fn demo_accounts(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        card(
            attrs: attributes! { class="w-full max-w-sm" },
            card_header(
                card_title((t!(demo_accounts)))
                // A message cannot contain markup, so the password follows the sentence.
                card_description(
                    (t!(demo_accounts.description))
                    " "
                    <code class="font-mono text-foreground">(DEMO_PASSWORD)</code>
                )
            )
            card_content(
                <ul class="flex flex-col gap-3 text-sm">
                    for (name, email, role) in DEMO_USERS {
                        <li class="flex items-center justify-between gap-4">
                            <div class="flex flex-col">
                                <span class="font-medium">(name)</span>
                                <span class="text-muted-foreground">(email)</span>
                            </div>
                            badge(variant: BadgeVariant::Outline, (role_label(role)))
                        </li>
                    }
                </ul>
            )
        )
    })
}
