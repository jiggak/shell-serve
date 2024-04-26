use std::{
    collections::HashMap, io::Read, os::fd::{AsRawFd, OwnedFd},
    path::{Component, Path}, process::{self, Stdio}, str::FromStr
};
use hyper::StatusCode;
use tokio::{io, process::{Child, Command}};

use crate::route_response::RouteResponse;
use super::Error;


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
    Named(String)
}

impl FromStr for RoutePart {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('{') {
            Ok(Self::Named(s[1..s.len()-1].to_string()))
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
    handler: String
}

impl FromStr for Route {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (method, path) = s.split_once(':')
            .ok_or(Error::InvalidRoute("missing method separator (:)".to_string()))?;

        let (path, handler) = path.split_once(' ')
            .ok_or(Error::InvalidRoute("missing handler separator (space)".to_string()))?;

        let path_parts = path.split_once('?');
        let (path, query) = if let Some((path, query)) = path_parts {
            let query: Result<Vec<_>, Error> = query.split('&')
                .into_iter()
                .map(|s| QueryPart::from_str(s))
                .collect();

            (path, Some(query?))
        } else {
            (path, None)
        };

        let path: Result<Vec<_>, Error> = path.split('/')
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| PathPart::from_str(s))
            .collect();

        Ok(Route {
            method: Method::from_str(method)?,
            path: path?,
            query,
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

    pub fn matches(&self, method: &Method, path: &Path, query: &HashMap<&str, &str>) -> Option<Vec<(&String, String)>> {
        if self.method != *method {
            return None;
        }

        // strip / prefix to match
        let path = if path.is_absolute() {
            path.strip_prefix("/").unwrap()
        } else {
            path
        };

        let mut iter_path = path.components();
        let mut iter_route_path = self.path.iter();

        let mut params = vec![];

        while let (Some(p), Some(r)) = (iter_path.next(), iter_route_path.next()) {
            let p = match p {
                Component::Normal(v) => v,
                _ => panic!("Unexpected path component variant")
            };

            let p = p.to_str().unwrap().to_string();

            match r {
                PathPart::Entry(RoutePart::Literal(v)) => {
                    if v != &p {
                        // literal route component doesn't match, return no-match
                        return None;
                    }
                },
                PathPart::Entry(RoutePart::Named(n)) => {
                    params.push((n, p))
                },
                PathPart::CatchAll(n) => {
                    let path = Path::new(&p).join(iter_path.as_path());
                    params.push((n, path.to_str().unwrap().to_string()))
                }
            }
        }

        if let Some(route_query) = &self.query {
            // clone query so we can remove matched entries, to get remainder for catch-all
            let mut query = query.clone();

            for entry in route_query {
                match entry {
                    QueryPart::KeyValue(route_query_k, route_query_v) => {
                        if let Some(value) = query.remove(route_query_k.as_str()) {
                            match route_query_v {
                                RoutePart::Literal(v) => {
                                    if v != value {
                                        // literal route query value doesn't match, return no-match
                                        return None;
                                    }
                                },
                                RoutePart::Named(n) => {
                                    params.push((&n, value.to_string()))
                                }
                            }
                        } else {
                            // key not found in query, return no-match
                            return None;
                        }
                    },
                    QueryPart::CatchAll(n) => {
                        let query_params = query.iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect::<Vec<_>>()
                            .join(",");

                        params.push((n, query_params))
                    }
                }
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
            process::id(),
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

pub struct RouteProcess {
    child: Child,
    read_pipe: os_pipe::PipeReader,
    write_pipe_fd: Option<OwnedFd>
}

impl RouteProcess {
    pub fn new(child: Child, read_pipe: os_pipe::PipeReader, write_pipe_fd: OwnedFd) -> Self {
        RouteProcess { child, read_pipe, write_pipe_fd: Some(write_pipe_fd) }
    }

    pub async fn load_stdin<S>(&mut self, reader: &mut S) -> Result<&mut Self, Error>
        where S: io::AsyncRead + Unpin
    {
        let mut stdin = self.child.stdin.take()
            .ok_or(Error::RouteIoOpen)?;

        io::copy(reader, &mut stdin).await?;

        Ok(self)
    }

    pub async fn wait(&mut self) -> Result<RouteResponse, Error> {
        let status = self.child.wait().await
            .map_err(|e| Error::RouteWait(e))?;

        // close writer side of pipe to avoid blocking reader
        let write_pipe_fd = self.write_pipe_fd.take()
            .expect("write_pipe_fd should be set");
        drop(write_pipe_fd);

        let mut pipe_buf = String::new();
        self.read_pipe.read_to_string(&mut pipe_buf)?;

        let headers = pipe_buf.lines()
            .map(parse_header)
            .collect::<Result<Vec<_>, _>>()?;

        let status = match headers.iter().find(|(k, _)| k == "Status") {
            Some((_, status)) => StatusCode::from_u16(
                status.parse().map_err(|_| Error::InvalidStatus(status.to_string()))?
            ).map_err(|_| Error::InvalidStatus(status.to_string()))?,
            None => match status.success() {
                true => StatusCode::OK,
                false => StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let stdout = self.child.stdout.take()
            .ok_or(Error::RouteIoOpen)?;

        Ok(RouteResponse::new(status, headers, stdout))
    }
}

fn parse_header(line: &str) -> Result<(String, String), Error> {
    let parts: Vec<_> = line.splitn(2, ':')
        .map(|s| s.trim())
        .collect();

    match &parts[..] {
        &[name, value, ..] => Ok((name.to_owned(), value.to_owned())),
        _ => Err(Error::InvalidHeader(line.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_route_match_literal() {
        let route = Route::from_str("GET:/foo/{file} handler.sh ${file}");
        assert!(route.is_ok());
        let route = route.unwrap();

        let query = HashMap::new();

        assert_eq!(
            route.matches(&Method::Get, Path::new("/foo/foo.txt"), &query),
            Some(vec![(&String::from("file"), String::from("foo.txt"))])
        );
        assert_eq!(
            route.matches(&Method::Get, Path::new("/bar/baz/foo.txt"), &query),
            None
        );
        assert_eq!(
            route.matches(&Method::Get, Path::new("/bar/foo.txt"), &query),
            None
        );
        assert_eq!(
            route.matches(&Method::Put, Path::new("/foo/foo.txt"), &query),
            None
        );
    }

    #[test]
    fn test_route_match_query() {
        let route = Route::from_str("GET:/{path..}?foo={foo}&{query..} handler.sh ${path} ${foo}");
        assert!(route.is_ok());
        let route = route.unwrap();

        let query = HashMap::from([("foo", "bar")]);

        assert_eq!(
            route.matches(&Method::Get, Path::new("/foo/bar/foo.txt"), &query),
            Some(vec![
                (&String::from("path"), String::from("foo/bar/foo.txt")),
                (&String::from("foo"), String::from("bar")),
                (&String::from("query"), String::from("")),
            ])
        );

        let query = HashMap::from([("foo", "bar"), ("baz", "foo")]);

        assert_eq!(
            route.matches(&Method::Get, Path::new("/foo/bar/foo.txt"), &query),
            Some(vec![
                (&String::from("path"), String::from("foo/bar/foo.txt")),
                (&String::from("foo"), String::from("bar")),
                (&String::from("query"), String::from("baz=foo")),
            ])
        );

        let query = HashMap::from([("baz", "foo")]);

        assert_eq!(
            route.matches(&Method::Get, Path::new("/foo/bar/foo.txt"), &query),
            None
        );
    }

    #[test]
    fn test_route_match_catchall() {
        let route = Route::from_str("GET:/{path..} handler.sh ${path}");
        assert!(route.is_ok());
        let route = route.unwrap();

        let query = HashMap::new();

        assert_eq!(
            route.matches(&Method::Get, Path::new("/foo/bar/foo.txt"), &query),
            Some(vec![(&String::from("path"), String::from("foo/bar/foo.txt"))])
        );
    }
}