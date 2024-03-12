#[macro_use] extern crate rocket;

use rocket::{data::{Data, ToByteUnit}, http::Status, State};
use shell_serve::{route::{Method, Route}, route_response::RouteResponse};
use shell_serve::router::{RouterError, ShellRouter};
use std::{collections::HashMap, path::PathBuf};


/*
This could be used to get full uri with query string, maybe?
Honestly I should concider just using hyper directly without rocket

use rocket::{http::uri::Origin, Request, request::{FromRequest, Outcome}};

pub struct RequestUri<'a>(pub Origin<'a>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequestUri<'r> {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(RequestUri(req.uri().to_owned()))
    }
}
*/

#[derive(Responder)]
struct RouteErrorResponse(Status);

impl From<RouterError> for RouteErrorResponse {
    fn from(value: RouterError) -> Self {
        match value {
            RouterError::RouteNotFound => RouteErrorResponse(Status::NotImplemented),
            RouterError::RouteSpawnFailed(_) => RouteErrorResponse(Status::InternalServerError)
        }
    }
}

impl From<shell_serve::Error> for RouteErrorResponse {
    fn from(_: shell_serve::Error) -> Self {
        RouteErrorResponse(Status::InternalServerError)
    }
}

type RouteResult = Result<RouteResponse, RouteErrorResponse>;

// #[get("/<_..>")]
#[get("/<path..>?<query..>")]
async fn _get(path: PathBuf, query: HashMap<&str, &str>, router: &State<ShellRouter>) -> RouteResult {
    let response = router.execute(&Method::Get, &path, &query)?
        .wait().await?;

    Ok(response)
}

#[put("/<path..>?<query..>", data = "<data>")]
async fn _put(path: PathBuf, data: Data<'_>, query: HashMap<&str, &str>, router: &State<ShellRouter>) -> RouteResult {
    let mut stream = data.open(10.megabytes());

    let response = router.execute(&Method::Put, &path, &query)?
        .load_stdin(&mut stream).await?
        .wait().await?;

    Ok(response)
}

#[post("/<path..>?<query..>", data = "<data>")]
async fn _post(path: PathBuf, data: Data<'_>, query: HashMap<&str, &str>, router: &State<ShellRouter>) -> RouteResult {
    let mut stream = data.open(10.megabytes());

    let response = router.execute(&Method::Post, &path, &query)?
        .load_stdin(&mut stream).await?
        .wait().await?;

    Ok(response)
}

#[delete("/<path..>?<query..>")]
async fn _delete(path: PathBuf, query: HashMap<&str, &str>, router: &State<ShellRouter>) -> RouteResult {
    let response = router.execute(&Method::Delete, &path, &query)?
        .wait().await?;

    Ok(response)
}

#[launch]
fn rocket() -> _ {
    let router = ShellRouter::new(vec![
        "GET:/{path..}?{query..} ./foo.sh ${path} ${query}".parse::<Route>().unwrap(),
        "PUT:/{path..} cat".parse::<Route>().unwrap()
    ]);

    rocket::build()
        .manage(router)
        .mount("/", routes![_get, _put, _post, _delete])
}

/*
#[rocket::main]
async fn main() {
    // Recall that an uninspected `Error` will cause a pretty-printed panic,
    // so rest assured errors do not go undetected when using `#[launch]`.
    println!("got here");
    let _ = rocket().launch().await;
}
*/