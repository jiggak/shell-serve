use std::{collections::HashMap, future::Future, path::Path, pin::Pin};
use http_body_util::combinators::BoxBody;
use hyper::{body, service::Service, Request, Response};

use crate::{route::Method, router::ShellRouter, Error};

impl Service<Request<body::Incoming>> for ShellRouter {
    type Response = Response<BoxBody<body::Bytes, std::io::Error>>;
    // type Response = Response<Full<body::Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    // type Future = Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>;

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
            let result = proc.wait()
                .await
                .unwrap();

            Ok(result.body())
        })
    }
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
