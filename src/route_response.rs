use rocket::request::Request;
use rocket::response::{self, Response, Responder};
use rocket::http::Status;
use tokio::process::ChildStdout;

pub struct RouteResponse {
    status: Status,
    headers: Vec<(String, String)>,
    stdout: ChildStdout
}

impl RouteResponse {
    pub fn new(status: Status, headers: Vec<(String, String)>, stdout: ChildStdout) -> Self {
        Self { status, headers, stdout }
    }
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for RouteResponse {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let mut response = Response::build();

        for (name, value) in self.headers {
            response.raw_header(name, value);
        }

        response.status(self.status)
            .streamed_body(self.stdout)
            .ok()
    }
}