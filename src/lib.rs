pub mod route;
pub mod router;
mod router_service;


#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid method '{0}'")]
    InvalidMethod(String),

    #[error("Invalid route: {0}")]
    InvalidRoute(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Invalid status code: {0}")]
    InvalidStatus(String),

    #[error("Failed to spawn route cmd")]
    RouteSpawn(#[source] std::io::Error),

    #[error("Failed to wait on route cmd")]
    RouteWait(#[source] std::io::Error),

    #[error("Route stream io error")]
    RouteIoError(#[from] std::io::Error),

    #[error("Failed to open route io stream")]
    RouteIoOpen
}
