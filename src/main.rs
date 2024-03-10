#[macro_use] extern crate rocket;

use rocket::{data::{Data, ToByteUnit}, http::Status, State};
use shell_serve::{route::{Method, Route}, route_response::RouteResponse};
use shell_serve::router::{RouterError, ShellRouter};
use std::path::PathBuf;


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

#[get("/<path..>")]
async fn _get(path: PathBuf, router: &State<ShellRouter>) -> RouteResult {
    let response = router.execute(&Method::Get, &path)?
        .wait().await?;

    Ok(response)
}

#[put("/<path..>", data = "<data>")]
async fn _put(path: PathBuf, data: Data<'_>, router: &State<ShellRouter>) -> RouteResult {
    let mut stream = data.open(10.megabytes());

    let response = router.execute(&Method::Put, &path)?
        .load_stdin(&mut stream).await?
        .wait().await?;

    Ok(response)
}

#[post("/<path..>", data = "<data>")]
async fn _post(path: PathBuf, data: Data<'_>, router: &State<ShellRouter>) -> RouteResult {
    let mut stream = data.open(10.megabytes());

    let response = router.execute(&Method::Post, &path)?
        .load_stdin(&mut stream).await?
        .wait().await?;

    Ok(response)
}

#[delete("/<path..>")]
async fn _delete(path: PathBuf, router: &State<ShellRouter>) -> RouteResult {
    let response = router.execute(&Method::Delete, &path)?
        .wait().await?;

    Ok(response)
}

#[launch]
fn rocket() -> _ {
    let router = ShellRouter::new(vec![
        "GET:/{path..}=./foo.sh ${path}".parse::<Route>().unwrap(),
        "PUT:/{path..}=cat".parse::<Route>().unwrap()
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