use crate::Error;
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use super::Method;

pub struct RouteRequest {
    pub method: Method,
    pub path: PathBuf,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>
}

impl FromStr for RouteRequest {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (method, path) = s.split_once(':')
            .ok_or(Error::InvalidRoute("missing method separator (:)".to_string()))?;

        let path_uri = urlparse::urlparse(path);
        let query = if let Some(query) = path_uri.get_parsed_query() {
            query.into_iter()
                .map(|(key, val)| (key, val.join(",")))
                .collect()
        } else {
            HashMap::new()
        };

        Ok(RouteRequest {
            method: Method::from_str(method)?,
            path: PathBuf::from(path_uri.path),
            query,
            headers: HashMap::new()
        })
    }
}