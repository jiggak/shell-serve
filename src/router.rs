use crate::route::{Route, RouteProcess, RouteRequest};


#[derive(Clone)]
pub struct ShellRouter {
    routes: Vec<Route>
}

impl ShellRouter {
    pub fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    pub fn execute(&self, req: &RouteRequest) -> Result<RouteProcess, RouterError> {
        let match_result = self.routes.iter().find_map(|r| {
            r.matches(&req).map(|m| (r, m))
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
    #[error("Unsupported method '{0}'")]
    UnsupportedMethod(String)
}
