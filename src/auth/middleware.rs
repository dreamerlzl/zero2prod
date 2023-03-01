use poem::{session::Session, Endpoint, Request, Result};
use uuid::Uuid;

use crate::{session_state::USER_ID_KEY, utils::see_other};

pub async fn reject_anoynmous_user<E: Endpoint>(next: E, mut req: Request) -> Result<E::Output> {
    let Some(Some(user_id)) = req.extensions().get::<Session>().map(|s| s.get::<Uuid>(USER_ID_KEY)) else {
        return Err(see_other("/login"));
    };
    req.extensions_mut().insert(user_id);
    let resp = next.call(req).await?;
    Ok(resp)
}
