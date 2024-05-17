mod process;
mod request;
mod response;
pub use process::RouteProcess;
pub use request::RouteRequest;
pub use response::RouteResponse;

use crate::Error;
use std::{
    collections::{HashMap, VecDeque}, os::fd::{AsRawFd, OwnedFd},
    path::{Component, Path}, process::Stdio, str::FromStr
};
use tokio::process::Command;


#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Method {
    Get,
    Put,
    Post,
    Delete
}

impl FromStr for Method {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Method::Get),
            "PUT" => Ok(Method::Put),
            "POST" => Ok(Method::Post),
            "DELETE" => Ok(Method::Delete),
            _ => Err(Error::InvalidMethod(s.to_string()))
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum RoutePart {
    Literal(String),
    Named(String),
    NamedOptional(String)
}

impl FromStr for RoutePart {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('{') {
            if s.ends_with("*}") {
                Ok(Self::NamedOptional(s[1..s.len()-2].to_string()))
            } else {
                Ok(Self::Named(s[1..s.len()-1].to_string()))
            }
        } else {
            Ok(Self::Literal(s.to_string()))
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum PathPart {
    Entry(RoutePart),
    CatchAll(String)
}

impl FromStr for PathPart {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        if s.starts_with('{') && s.ends_with("..}") {
            Ok(Self::CatchAll(s[1..s.len()-3].to_string()))
        } else {
            Ok(Self::Entry(s.parse()?))
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum QueryPart {
    KeyValue(String, RoutePart),
    CatchAll(String)
}

impl FromStr for QueryPart {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('{') && s.ends_with("..}") {
            Ok(Self::CatchAll(s[1..s.len()-3].to_string()))
        } else {
            let (name, value) = s.split_once('=')
                .ok_or(Error::InvalidRoute("invalid key=value entry in query".to_string()))?;

            Ok(Self::KeyValue(name.to_string(), value.parse()?))
        }
    }
}

#[derive(Debug, Clone)]
pub struct Route {
    method: Method,
    path: Vec<PathPart>,
    query: Option<Vec<QueryPart>>,
    headers: Option<Vec<QueryPart>>,
    handler: String
}

impl FromStr for Route {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (method, path) = s.split_once(':')
            .ok_or(Error::InvalidRoute("missing method separator (:)".to_string()))?;

        let (path, handler) = path.split_once(' ')
            .ok_or(Error::InvalidRoute("missing handler separator (space)".to_string()))?;

        let path_uri = urlparse::urlparse(path);

        let path = path_uri.path.split('/')
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| PathPart::from_str(s))
            .collect::<Result<Vec<_>, _>>();

        let query = if let Some(query) = path_uri.query {
            let query = query.split('&')
                .into_iter()
                .map(|s| QueryPart::from_str(s))
                .collect::<Result<Vec<_>, _>>();

            Some(query?)
        } else {
            None
        };

        let headers = if let Some(fragment) = path_uri.fragment {
            let headers = fragment.split('&')
                .into_iter()
                .map(|s| QueryPart::from_str(s))
                .collect::<Result<Vec<_>, _>>();

            Some(headers?)
        } else {
            None
        };

        Ok(Route {
            method: Method::from_str(method)?,
            path: path?,
            query,
            headers,
            handler: handler.to_string()
        })
    }
}

impl Route {
    pub fn get_command(&self, params: Vec<(&String, String)>) -> Result<Command, Error> {
        let mut parts = self.handler.split(' ');

        let cmd = parts.next()
            .ok_or(Error::InvalidRoute("missing handler cmd".to_string()))?;

        let mut cmd = Command::new(cmd);

        let ctx: HashMap<_, _> = params.into_iter()
            .collect();

        for arg in parts {
            cmd.arg(
                shellexpand::env_with_context_no_errors(arg,
                    |var| ctx.get(&String::from(var))
                ).to_string()
            );
        }

        Ok(cmd)
    }

    pub fn matches(&self, req: &RouteRequest) -> Option<Vec<(&String, String)>> {
        if self.method != req.method {
            return None;
        }

        // strip / prefix to match
        let path = if req.path.is_absolute() {
            req.path.strip_prefix("/").unwrap()
        } else {
            req.path.as_ref()
        };

        let mut params = vec![];

        let result = PathMatchIterator::new(self.path.iter(), path)
            .matches();
        if let Some(matches) = result {
            params.extend(matches);
        } else {
            return None;
        }

        if let Some(route_query) = &self.query {
            let result = QueryMatchIterator::new(route_query.iter(), &req.query)
                .matches();
            if let Some(matches) = result {
                params.extend(matches);
            } else {
                return None;
            }
        }

        if let Some(route_headers) = &self.headers {
            let result = QueryMatchIterator::new(route_headers.iter(), &req.headers)
                .matches();
            if let Some(matches) = result {
                params.extend(matches);
            } else {
                return None;
            }
        }

        Some(params)
    }

    pub fn spawn(&self, params: Vec<(&String, String)>) -> Result<RouteProcess, Error> {
        let mut cmd = self.get_command(params)?;

        let (read_pipe, write_pipe) = os_pipe::pipe()?;
        let write_pipe_fd: OwnedFd = write_pipe.into();

        // FIXME could this be made cross platform, or at least work on MacOS?
        let write_pipe_path = format!("/proc/{}/fd/{}",
            std::process::id(),
            write_pipe_fd.as_raw_fd()
        );

        cmd.env("SHELL_SERVE_PIPE", write_pipe_path);

        let child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::RouteSpawn(e))?;

        Ok(RouteProcess::new(child, read_pipe, write_pipe_fd))
    }
}

enum MatchResult<'a> {
    Match(&'a String, String),
    MatchLiteral,
    NoMatch
}

struct QueryMatchIterator<I> {
    iter: I,
    haystack: HashMap<String, String>
}

impl<'a, I> QueryMatchIterator<I>
where
    I: Iterator<Item = &'a QueryPart>
{
    fn new(iter: I, haystack: &HashMap<String, String>) -> Self {
        // clone so we can remove matched entries, to get remainder for catch-all
        QueryMatchIterator { iter, haystack: haystack.clone() }
    }
}

impl<'a, I> Iterator for QueryMatchIterator<I>
where
    I: Iterator<Item = &'a QueryPart>
{
    type Item = MatchResult<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.iter.next() {
            match entry {
                QueryPart::KeyValue(entry_k, entry_v) => {
                    if let Some(value) = self.haystack.remove(entry_k.as_str()) {
                        match entry_v {
                            RoutePart::Literal(v) => {
                                if *v != value {
                                    // literal route query value doesn't match, return no-match
                                    Some(MatchResult::NoMatch)
                                } else {
                                    Some(MatchResult::MatchLiteral)
                                }
                            },
                            RoutePart::Named(n) => {
                                Some(MatchResult::Match(n, value.to_string()))
                            },
                            RoutePart::NamedOptional(n) => {
                                Some(MatchResult::Match(n, value.to_string()))
                            }
                        }
                    } else { // key not found in haystack
                        match entry_v {
                            RoutePart::NamedOptional(n) => {
                                Some(MatchResult::Match(n, String::new()))
                            },
                            _ => Some(MatchResult::NoMatch)
                        }
                    }
                },
                QueryPart::CatchAll(n) => {
                    let query_params = self.haystack.drain()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join("&");

                    Some(MatchResult::Match(n, query_params))
                }
            }
        } else {
            None
        }
    }
}

struct PathMatchIterator<I> {
    iter: I,
    haystack: VecDeque<String>
}

impl<'a, I> PathMatchIterator<I>
where
    I: Iterator<Item = &'a PathPart>
{
    fn new(iter: I, path: &Path) -> Self {
        let haystack = path.components()
            .map(|c| match c {
                Component::Normal(v) => v.to_str().unwrap().to_string(),
                _ => panic!("Unexpected path component variant")
            })
            .collect();
        PathMatchIterator { iter, haystack }
    }
}

impl<'a, I> Iterator for PathMatchIterator<I>
where
    I: Iterator<Item = &'a PathPart>
{
    type Item = MatchResult<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let path_part = if let Some(path_part) = self.iter.next() {
            path_part
        } else {
            return None;
        };

        match path_part {
            PathPart::Entry(entry) => {
                let path_entry = self.haystack.pop_front();
                match entry {
                    RoutePart::Literal(v) => {
                        if let Some(val) = path_entry {
                            if v == &val {
                                Some(MatchResult::MatchLiteral)
                            } else {
                                Some(MatchResult::NoMatch)
                            }
                        } else {
                            Some(MatchResult::NoMatch)
                        }
                    },
                    RoutePart::Named(n) => {
                        if let Some(val) = path_entry {
                            Some(MatchResult::Match(n, val))
                        } else {
                            Some(MatchResult::NoMatch)
                        }
                    },
                    RoutePart::NamedOptional(n) => {
                        let val = path_entry.unwrap_or(String::new());
                        Some(MatchResult::Match(n, val))
                    }
                }
            },
            PathPart::CatchAll(n) => {
                let remaining: Vec<_> = self.haystack.drain(..).collect();
                return Some(MatchResult::Match(n, remaining.join("/")));
            }
        }
    }
}

trait MatchIterator<'a>: Iterator<Item = MatchResult<'a>> {
    fn matches(&mut self) -> Option<Vec<(&'a String, String)>> {
        let mut matches = vec![];

        while let Some(x) = self.next() {
            match x {
                MatchResult::Match(k, v) => matches.push((k, v)),
                MatchResult::MatchLiteral => (),
                MatchResult::NoMatch => return None
            }
        }

        if self.haystack_count() != 0 {
            return None;
        }

        return Some(matches);
    }

    fn haystack_count(&self) -> usize;
}

impl<'a, I> MatchIterator<'a> for QueryMatchIterator<I>
where I: Iterator<Item = &'a QueryPart> {
    fn haystack_count(&self) -> usize {
        self.haystack.len()
    }
}

impl<'a, I> MatchIterator<'a> for PathMatchIterator<I>
where I: Iterator<Item = &'a PathPart> {
    fn haystack_count(&self) -> usize {
        self.haystack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl RouteRequest {
        fn with_headers<H>(mut self, headers: H) -> Self
            where H: Into<HashMap<String, String>>
        {
            self.headers = headers.into();
            self
        }
    }

    #[test]
    fn test_route_parse() {
        let route = Route::from_str("GET:/foo/{file} handler_get_foo.sh ${file}");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(route.method, Method::Get);
        assert_eq!(route.path, vec![
            PathPart::Entry(RoutePart::Literal("foo".to_string())),
            PathPart::Entry(RoutePart::Named("file".to_string()))
        ]);
        assert_eq!(route.handler, "handler_get_foo.sh ${file}");
    }

    #[test]
    fn test_route_parse_optional() {
        let route = Route::from_str("GET:/{file*}?bar={foo*} handler.sh ${file} ${foo}");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(route.path, vec![
            PathPart::Entry(RoutePart::NamedOptional("file".to_string()))
        ]);
        assert_eq!(route.query, Some(vec![
            QueryPart::KeyValue("bar".to_string(), RoutePart::NamedOptional("foo".to_string()))
        ]));
    }

    #[test]
    fn test_route_match_literal() {
        let route = Route::from_str("GET:/foo/{file} handler.sh ${file}");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(
            route.matches(&"GET:/foo/foo.txt".parse().unwrap()),
            Some(vec![(&String::from("file"), String::from("foo.txt"))])
        );
        assert_eq!(
            route.matches(&"GET:/bar/baz/foo.txt".parse().unwrap()),
            None
        );
        assert_eq!(
            route.matches(&"GET:/bar/foo.txt".parse().unwrap()),
            None
        );
        assert_eq!(
            route.matches(&"PUT:/foo/foo.txt".parse().unwrap()),
            None
        );
    }

    #[test]
    fn test_route_match_query() {
        let route = Route::from_str("GET:/{path..}?foo={foo}&{query..} handler.sh ${path} ${foo}");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(
            route.matches(&"GET:/foo/bar/foo.txt?foo=bar".parse().unwrap()),
            Some(vec![
                (&String::from("path"), String::from("foo/bar/foo.txt")),
                (&String::from("foo"), String::from("bar")),
                (&String::from("query"), String::from("")),
            ])
        );

        assert_eq!(
            route.matches(&"GET:/foo/bar/foo.txt?foo=bar&baz=foo".parse().unwrap()),
            Some(vec![
                (&String::from("path"), String::from("foo/bar/foo.txt")),
                (&String::from("foo"), String::from("bar")),
                (&String::from("query"), String::from("baz=foo")),
            ])
        );

        assert_eq!(
            route.matches(&RouteRequest::from_str("GET:/foo/bar/foo.txt?baz=foo").unwrap()),
            None
        );
    }

    #[test]
    fn test_route_match_catchall() {
        let route = Route::from_str("GET:/{path..} handler.sh ${path}");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(
            route.matches(&"GET:/foo/bar/foo.txt".parse().unwrap()),
            Some(vec![(&String::from("path"), String::from("foo/bar/foo.txt"))])
        );
    }

    #[test]
    fn test_route_match_root() {
        let route = Route::from_str("GET:/ handler.sh");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(
            route.matches(&"GET:/".parse().unwrap()),
            Some(vec![])
        );

        assert_eq!(
            route.matches(&"GET:/file.txt".parse().unwrap()),
            None
        );
    }

    #[test]
    fn test_route_match_short_path() {
        let route = Route::from_str("GET:/{path..}?{query..} handler.sh ${path} ${query}");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(
            // route.matches(&RouteRequest::from_str("GET:/foo").unwrap()),
            route.matches(&"GET:/foo".parse().unwrap()),
            Some(vec![
                (&String::from("path"), String::from("foo")),
                (&String::from("query"), String::from(""))
            ])
        );
    }

    #[test]
    fn test_route_match_headers() {
        let route = Route::from_str("GET:/{path..}?{query..}#{headers..} handler.sh ${path} ${query} ${headers}");
        assert!(route.is_ok());
        let route = route.unwrap();

        let req = RouteRequest::from_str("GET:/foo.txt?foo=bar").unwrap()
            .with_headers([("X-Foo".to_string(), "bar".to_string())]);

        assert_eq!(
            route.matches(&req),
            Some(vec![
                (&String::from("path"), String::from("foo.txt")),
                (&String::from("query"), String::from("foo=bar")),
                (&String::from("headers"), String::from("X-Foo=bar"))
            ])
        );
    }

    #[test]
    fn test_route_match_optional() {
        let route = Route::from_str("GET:/{file*}?param={val*} handler.sh ${file} ${val}");
        assert!(route.is_ok());
        let route = route.unwrap();

        let req = RouteRequest::from_str("GET:/").unwrap();

        assert_eq!(
            route.matches(&req),
            Some(vec![
                (&String::from("file"), String::from("")),
                (&String::from("val"), String::from(""))
            ])
        );
    }
}