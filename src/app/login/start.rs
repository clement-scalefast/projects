use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        route,
    },
};

use crate::features::keycloak::start_sign_in;

/// Sends the browser to Keycloak to sign in.
#[route(GET)]
pub async fn start(cx: &Cx) -> Result<SeeOther> {
    Ok(see_other(start_sign_in(cx)?))
}
