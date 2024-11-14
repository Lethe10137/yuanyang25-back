pub trait APIRequest: Sized {
    fn sanity(self) -> Option<Self>;
}

#[macro_export]
macro_rules! sanity {
    ($form:expr) => {
        match $form.into_inner().sanity() {
            Some(valid_form) => valid_form,
            None => return HttpResponse::BadRequest().body("Invalid form data"),
        }
    };
}

#[macro_export]
macro_rules! internal_error {
    ($e:expr, $msg:expr) => {{
        error!("{}", $e);
        return HttpResponse::InternalServerError().body(format!(
            "System error {}! Contact ch-li21@mails.tsinghua.edu.cn.",
            $msg
        ));
    }};
}

pub static SESSION_USER_ID: &str = "user_id";
pub static SESSION_PRIVILEDGE: &str = "user_priviledge";
