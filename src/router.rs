use std::path::Path;
use crate::route::{Method, Route, RouteOutput};

pub struct ShellRouter {
    routes: Vec<Route>
}

impl ShellRouter {
    pub fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    pub fn execute(&self, method: &Method, path: &Path) -> Result<RouteOutput, RouterError> {
        let match_result = self.routes.iter().find_map(|r| {
            r.matches(method, path)
                .map(|m| (r, m))
        });

        if let Some((route, params)) = match_result {
            Ok(route.execute(params)
                .map_err(|e| RouterError::RouteSpawnFailed(e))?)
        } else {
            Err(RouterError::RouteNotFound)
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum RouterError {
    #[error("No matching route found")]
    RouteNotFound,
    #[error("Route command failed to spawn")]
    RouteSpawnFailed(#[from] crate::Error),
}
