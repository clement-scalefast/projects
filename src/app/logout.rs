use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        route,
    },
};

use crate::features::{auth::sign_out, keycloak::sign_out_url};

/// Ends the app's session, then sends the browser to Keycloak to end its session
/// there. Keycloak sends the browser back to the sign-in page.
#[route(POST)]
pub async fn submit(cx: &Cx) -> Result<SeeOther> {
    let id_token = sign_out(cx).await?;
    Ok(see_other(sign_out_url(cx, id_token.as_deref())?))
}
