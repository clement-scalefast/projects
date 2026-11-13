use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        href, query_params, route,
    },
};

use crate::{
    app::_workspace::_dashboard,
    features::{auth::sign_in, keycloak::finish_sign_in, user::User},
};

/// What Keycloak adds to the URL when it sends the browser back.
#[query_params(error = bad_request)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

/// Completes a sign-in when Keycloak sends the browser back. Returns to the
/// sign-in page with a message if the sign-in was cancelled or is not valid.
#[route(GET)]
pub async fn callback(cx: &Cx) -> Result<SeeOther> {
    let query = query_params::<CallbackQuery>(cx)?;
    let identity = match (&query.code, &query.state) {
        (Some(code), Some(state)) => finish_sign_in(cx, code, state).await?,
        _ => None,
    };
    let Some(identity) = identity else {
        let login = href!(super::page).resolve(cx);
        return Ok(see_other(format!("{login}?failed=1")));
    };

    let user = User::from_identity(cx, &identity).await?;
    sign_in(cx, &user, &identity.id_token).await?;
    Ok(see_other(href!(_dashboard::page).resolve(cx)))
}
