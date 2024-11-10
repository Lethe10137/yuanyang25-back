use actix_web::{get, HttpResponse, Responder};

use actix_session::Session;
use log::error;

use crate::api_util::SESSION_VERIFY;
use crate::cipher_util;

// [[API]]
// Description: Login with password.
// Method: Post
// URL: /vericode
// Request Body: N/A
// Response Body: A string, the vericode.
//
#[get("/vericode")]
async fn vericode(session: Session) -> impl Responder {
    let vericode = cipher_util::gen_verify_session();
    match session.insert(SESSION_VERIFY, vericode.as_str()) {
        Ok(_) => HttpResponse::Ok().json(vericode),
        Err(e) => internal_error!(e, "vericode"),
    }
}
