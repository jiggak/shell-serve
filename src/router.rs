use std::path::Path;
use crate::route::{Method, Route};

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
