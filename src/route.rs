use std::{
    collections::HashMap, io::Read, os::fd::{AsRawFd, OwnedFd},
    path::{Component, Path}, process::{self, Stdio}, str::FromStr
};
use rocket::http::{Header, HeaderMap, Status};
use tokio::{io, process::{Child, Command}};
use super::Error;


#[derive(Debug, Eq, PartialEq)]
pub enum Method {
    Get,
    Put,
    Delete
}

impl FromStr for Method {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Method::Get),
            "PUT" => Ok(Method::Put),
            "DELETE" => Ok(Method::Delete),
            _ => Err(Error::InvalidMethod(s.to_string()))
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum PathPart {
    Literal(String),
    Named(String),
    CatchAll(String)
}

impl FromStr for PathPart {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        if s.starts_with('{') {
            if s.ends_with("..}") {
                Ok(PathPart::CatchAll(s[1..s.len()-3].to_string()))
            } else if s.ends_with('}') {
                Ok(PathPart::Named(s[1..s.len()-1].to_string()))
            } else {
                Err(Error::InvalidPathPart(s.to_string()))
            }
        } else {
            Ok(PathPart::Literal(s.to_string()))
        }
    }
}

#[derive(Debug)]
pub struct Route {
    method: Method,
    path: Vec<PathPart>,
    handler: String
}

impl FromStr for Route {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (method, path) = s.split_once(':')
            .ok_or(Error::InvalidRoute("missing :".to_string()))?;

        let (path, handler) = path.split_once('=')
            .ok_or(Error::InvalidRoute("missing =".to_string()))?;

        let path: Result<Vec<_>, Error> = path.split('/')
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| PathPart::from_str(s))
            .collect();

        Ok(Route {
            method: Method::from_str(method)?,
            path: path?,
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

    pub fn matches(&self, method: &Method, path: &Path) -> Option<Vec<(&String, String)>> {
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
                PathPart::Literal(v) => {
                    if v != &p {
                        return None;
                    }
                },
                PathPart::Named(n) => {
                    params.push((n, p))
                },
                PathPart::CatchAll(n) => {
                    let path = Path::new(&p).join(iter_path.as_path());
                    params.push((n, path.to_str().unwrap().to_string()))
                }
            }
        }

        Some(params)
    }

    pub fn spawn(&self, params: Vec<(&String, String)>) -> Result<RouteOutput, Error> {
        let mut cmd = self.get_command(params)?;

        let (read_pipe, write_pipe) = os_pipe::pipe()?;
        let write_pipe_fd: OwnedFd = write_pipe.into();

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

        Ok(RouteOutput::new(child, read_pipe, write_pipe_fd))
    }
}

pub struct RouteOutput {
    child: Child,
    read_pipe: os_pipe::PipeReader,
    write_pipe_fd: OwnedFd
}

impl RouteOutput {
    pub fn new(child: Child, read_pipe: os_pipe::PipeReader, write_pipe_fd: OwnedFd) -> Self {
        RouteOutput { child, read_pipe, write_pipe_fd }
    }

    pub fn stdout(&mut self) -> Result<impl io::AsyncRead, Error> {
        Ok(self.child.stdout.take()
            .ok_or(Error::RouteIoOpen)?)
    }

    pub async fn write_stdin<S>(&mut self, reader: &mut S) -> Result<u64, Error>
        where S: io::AsyncRead + Unpin
    {
        let stdin = self.child.stdin.take()
        .ok_or(Error::RouteIoOpen)?;

        let mut writer = io::BufWriter::new(stdin);

        Ok(io::copy(reader, &mut writer).await?)
    }

    pub async fn wait(mut self) -> Result<Status, Error> {
        let status = self.child.wait().await
            .map_err(|e| Error::RouteWait(e))?;

        // close writer side to avoid blocking reader
        drop(self.write_pipe_fd);

        let mut pipe_buf = String::new();
        self.read_pipe.read_to_string(&mut pipe_buf)?;

        let headers = pipe_buf.lines()
            .map(parse_header)
            .collect::<Result<Vec<_>, _>>()?;

        let mut header_map = HeaderMap::new();
        for header in headers {
            header_map.add(header);
        }

        let status = match header_map.get_one("Status") {
            Some(status) => Status::from_code(
                status.parse().map_err(|_| Error::InvalidStatus(status.to_string()))?
            ).ok_or(Error::InvalidStatus(status.to_string()))?,
            None => match status.success() {
                true => Status::Ok,
                false => Status::InternalServerError
            }
        };

        Ok(status)
    }
}

fn parse_header(line: &str) -> Result<Header, Error> {
    let parts: Vec<_> = line.splitn(2, ':')
        .map(|s| s.trim())
        .collect();

    match &parts[..] {
        &[name, value, ..] => Ok(Header::new(name, value)),
        _ => Err(Error::InvalidHeader(line.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_parse() {
        let route = Route::from_str("GET:/foo/{file}=handler_get_foo.sh ${file}");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(route.method, Method::Get);
        assert_eq!(route.path, vec![
            PathPart::Literal("foo".to_string()),
            PathPart::Named("file".to_string())
        ]);
        assert_eq!(route.handler, "handler_get_foo.sh ${file}");
    }

    #[test]
    fn test_route_match() {
        let route = Route::from_str("GET:/foo/{file}=handler.sh ${file}");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(
            route.matches(&Method::Get, Path::new("/foo/foo.txt")),
            Some(vec![(&String::from("file"), String::from("foo.txt"))])
        );
        assert_eq!(
            route.matches(&Method::Get, Path::new("/bar/baz/foo.txt")),
            None
        );
        assert_eq!(
            route.matches(&Method::Get, Path::new("/bar/foo.txt")),
            None
        );
        assert_eq!(
            route.matches(&Method::Put, Path::new("/foo/foo.txt")),
            None
        );

        let route = Route::from_str("GET:/{path..}=handler.sh ${path}");
        assert!(route.is_ok());
        let route = route.unwrap();

        assert_eq!(
            route.matches(&Method::Get, Path::new("/foo/bar/foo.txt")),
            Some(vec![(&String::from("path"), String::from("foo/bar/foo.txt"))])
        );
    }
}