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
