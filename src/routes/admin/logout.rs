use actix_web::web;
use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::authentication::UserId;
use crate::session_state::TypedSession;
use crate::utils::see_other;

pub async fn log_out(
    session: TypedSession,
    _user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    session.log_out();
    FlashMessage::info("You have successfully logged out.").send();
    Ok(see_other("/login"))
}
