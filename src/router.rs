use crate::route::{Method, Route, RouteProcess};
use std::{collections::HashMap, path::Path};


#[derive(Clone)]
pub struct ShellRouter {
    routes: Vec<Route>
}

impl ShellRouter {
    pub fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    pub fn execute(&self, method: &Method, path: &Path, query: &HashMap<&str, &str>) -> Result<RouteProcess, RouterError> {
        let match_result = self.routes.iter().find_map(|r| {
            r.matches(method, path, query)
                .map(|m| (r, m))
        });

        if let Some((route, params)) = match_result {
            Ok(route.spawn(params)?)
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
