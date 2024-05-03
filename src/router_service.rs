use crate::{
    route::{Method, RouteRequest, RouteResponse},
    router::{RouterError, ShellRouter}
};
use futures_util::TryStreamExt;
use http_body_util::{combinators::BoxBody, BodyExt, BodyStream, Empty, StreamBody};
use hyper::{body, body::Body, StatusCode, Request, Response};
use std::{collections::HashMap, io::Error as IoError, str::FromStr, path::PathBuf};
use tokio::io::AsyncRead;
use tokio_util::io::{ReaderStream, StreamReader};


type ServiceResponse = Response<BoxBody<body::Bytes, IoError>>;

impl ShellRouter {
    pub async fn call(&self, req: Request<body::Incoming>) -> Result<ServiceResponse, RouterError> {
        let result = self._call(req).await;
        match result {
            Ok(response) => Ok(response),
            Err(RouterError::UnsupportedMethod(_)) => {
                Ok(empty_response(StatusCode::METHOD_NOT_ALLOWED))
            },
            Err(RouterError::RouteNotFound) => {
                Ok(empty_response(StatusCode::NOT_FOUND))
            },
            Err(e) => Err(e)
        }
    }

    async fn _call(&self, req: Request<body::Incoming>) -> Result<ServiceResponse, RouterError> {
        let route_req = to_route_req(&req)?;

        let mut proc = self.execute(&route_req)?;

        let proc = if req.body().size_hint().upper().unwrap_or(0) > 0 {
            let stream_reader = body_stream_reader(req.into_body());
            let mut stream_reader = std::pin::pin!(stream_reader);

            proc.load_stdin(&mut stream_reader)
                .await?
        } else {
            &mut proc
        };

        let result = proc.wait()
            .await?;

        Ok(route_response(result))
    }
}

fn empty_response(status: StatusCode) -> ServiceResponse {
    let empty_body = Empty::<body::Bytes>::new()
        .map_err(|never| match never {});

    Response::builder()
        .status(status)
        .body(empty_body.boxed())
        .unwrap()
}

fn body_stream_reader(body: body::Incoming) -> impl AsyncRead {
    let stream_of_frames = BodyStream::new(body);
    let stream_of_bytes = stream_of_frames
        .try_filter_map(|frame| async move { Ok(frame.into_data().ok()) })
        .map_err(|err| IoError::new(std::io::ErrorKind::Other, err));
    StreamReader::new(stream_of_bytes)
}

fn to_route_req(req: &Request<body::Incoming>) -> Result<RouteRequest, RouterError> {
    let method = req.method().as_str();
    let method = Method::from_str(method)
        .map_err(|_| RouterError::UnsupportedMethod(method.into()))?;

    let path = PathBuf::from(req.uri().path());

    let query = if let Some(query) = req.uri().query() {
        urlparse::parse_qs(query).into_iter()
            .map(|(key, val)| (key, val.join(",")))
            .collect()
    } else {
        HashMap::new()
    };

    Ok(RouteRequest { method, path, query })
}

fn route_response(res: RouteResponse) -> Response<BoxBody<body::Bytes, IoError>> {
    let reader_stream = ReaderStream::new(res.stdout);
    let stream_body = StreamBody::new(reader_stream.map_ok(body::Frame::data));
    let boxed_body = stream_body.boxed();

    let mut builder = Response::builder();

    for (name, value) in res.headers {
        builder = builder.header(name, value);
    }

    builder.status(res.status)
        //.body(StreamBody::new(self.stdout))
        .body(boxed_body)
        .unwrap()
}
