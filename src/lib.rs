pub mod route;
pub mod router;


#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid method '{0}'")]
    InvalidMethod(String),
    #[error("Invalid path part '{0}'")]
    InvalidPathPart(String),
    #[error("Invalid route: {0}")]
    InvalidRoute(String),
    #[error("Failed to spawn route cmd")]
    RouteSpawn(#[from] std::io::Error)
}
