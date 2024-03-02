use std::{
    io::Read, path::{Component, Path}, process::{Command, Stdio}, str::FromStr
};


#[derive(Debug, Eq, PartialEq)]
pub enum Method {
    Get,
    Put,
    Delete
}

impl FromStr for Method {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Method::Get),
            "PUT" => Ok(Method::Put),
            "DELETE" => Ok(Method::Delete),
            _ => Err(format!("Unexpected '{s}' for Method")),
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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('{') {
            if s.ends_with("..}") {
                Ok(PathPart::CatchAll(s[1..s.len()-3].to_string()))
            } else if s.ends_with('}') {
                Ok(PathPart::Named(s[1..s.len()-1].to_string()))
            } else {
                Err(format!("Invalid path component {s}"))
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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (method, path) = s.split_once(':')
            .ok_or("Missing : in route".to_string())?;

        let (path, handler) = path.split_once('=')
            .ok_or("Missing = in route".to_string())?;

        let path: Result<Vec<_>, String> = path.split('/')
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
    pub fn get_command(&self, params: Vec<(&String, String)>) -> Result<Command, String> {
        let mut parts = self.handler.split(' ');

        let cmd = parts.next().unwrap();
        let mut cmd = Command::new(cmd);

        // TODO variable subst
        cmd.args(parts);

        Ok(cmd)
    }

    //pub fn matches<P: AsRef<Path>>(&self, method: &Method, path: P) -> bool {
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

    pub fn execute(&self, params: Vec<(&String, String)>) -> Result<String, String> {
        let mut cmd = self.get_command(params)?;

        // TODO error handling
        let mut child = cmd
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to execute child");

        let mut stdout = child.stdout
            .take()
            .expect("failed to take child stdout");

        let mut buf = String::new();
        stdout.read_to_string(&mut buf)
            .expect("failed to read string from stdout");
        Ok(buf)
    }
}

pub struct ShellRouter {
    routes: Vec<Route>
}

impl ShellRouter {
    pub fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    pub fn execute(&self, method: &Method, path: &Path) -> String {
        let match_result = self.routes.iter().find_map(|r| {
            r.matches(method, path)
                .map(|m| (r, m))
        });

        if let Some((route, params)) = match_result {
            route.execute(params).unwrap()
        } else {
            "Uh, Oh! It broke!".to_string()
        }
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