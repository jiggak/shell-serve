use http_body_util::{combinators::BoxBody, BodyExt, StreamBody};
use hyper::{body::{Bytes, Frame}, Response, StatusCode};
use tokio::process::ChildStdout;
use tokio_util::io::ReaderStream;
use futures_util::TryStreamExt;

pub struct RouteResponse {
    status: StatusCode,
    headers: Vec<(String, String)>,
    stdout: ChildStdout
}

impl RouteResponse {
    pub fn new(status: StatusCode, headers: Vec<(String, String)>, stdout: ChildStdout) -> Self {
        Self { status, headers, stdout }
    }

    pub fn body(self) -> Response<BoxBody<Bytes, std::io::Error>> {
        let reader_stream = ReaderStream::new(self.stdout);
        let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
        let boxed_body = stream_body.boxed();

        let mut builder = Response::builder();

        for (name, value) in self.headers {
            builder = builder.header(name, value);
        }

        builder.status(self.status)
            //.body(StreamBody::new(self.stdout))
            .body(boxed_body)
            .unwrap()
    }
}
