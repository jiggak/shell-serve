use futures_util::TryStreamExt;
use http_body_util::{combinators::BoxBody, BodyStream};
use hyper::body::Body;
use hyper::{body, service::Service, Request, Response};
use std::{collections::HashMap, future::Future, io::Error as IoError, path::Path, pin::Pin};
use tokio::io::AsyncRead;
use tokio_util::io::StreamReader;

use crate::{route::Method, router::ShellRouter, Error};


impl Service<Request<body::Incoming>> for ShellRouter {
    type Response = Response<BoxBody<body::Bytes, IoError>>;
    // type Response = Response<Full<body::Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<body::Incoming>) -> Self::Future {
        // let url = urlparse::urlparse(req.uri().into());
        let method = req.method().try_into().unwrap();

        let path = Path::new(req.uri().path());
        let query = if let Some(query) = req.uri().query() {
            urlparse::parse_qs(query).into_iter()
                .map(|(key, val)| (key, val.join(",")))
                .collect()
        } else {
            HashMap::new()
        };

        let query = query.iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
            .collect();

        let mut proc = self.execute(&method, &path, &query)
            .unwrap();

        Box::pin(async move {
            let proc = if req.body().size_hint().upper().unwrap_or(0) > 0 {
                let stream_reader = body_stream_reader(req.into_body());
                let mut stream_reader = std::pin::pin!(stream_reader);

                proc.load_stdin(&mut stream_reader)
                    .await.unwrap()
            } else {
                &mut proc
            };

            let result = proc.wait()
                .await.unwrap();

            Ok(result.body())
        })
    }
}

fn body_stream_reader(body: body::Incoming) -> impl AsyncRead {
    let stream_of_frames = BodyStream::new(body);
    let stream_of_bytes = stream_of_frames
        .try_filter_map(|frame| async move { Ok(frame.into_data().ok()) })
        .map_err(|err| IoError::new(std::io::ErrorKind::Other, err));
    StreamReader::new(stream_of_bytes)
}

impl TryFrom<&hyper::Method> for Method {
    type Error = crate::Error;
    fn try_from(value: &hyper::Method) -> Result<Self, Self::Error> {
        match *value {
            hyper::Method::GET => Ok(Method::Get),
            hyper::Method::PUT => Ok(Method::Put),
            hyper::Method::POST => Ok(Method::Post),
            hyper::Method::DELETE => Ok(Method::Delete),
            _ => Err(Error::UnsupportedMethod)
        }
    }
}
