use hyper::StatusCode;
use tokio::process::ChildStdout;

pub struct RouteResponse {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub stdout: ChildStdout
}