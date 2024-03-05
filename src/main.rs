#[macro_use] extern crate rocket;

use rocket::{data::{Data, ToByteUnit}, http::Status, State};
use rocket::response::stream::ReaderStream;
use rocket::response::status;
use rocket::tokio::io::AsyncRead;
use shell_serve::route::{Method, Route};
use shell_serve::router::{RouterError, ShellRouter};
use std::path::PathBuf;


#[derive(Responder)]
struct RouteErrorResponse(Status);

impl From<RouterError> for RouteErrorResponse {
    fn from(value: RouterError) -> Self {
        match value {
            RouterError::RouteNotFound => RouteErrorResponse(Status::NotFound),
            RouterError::RouteSpawnFailed(_) => RouteErrorResponse(Status::InternalServerError)
        }
    }
}

impl From<shell_serve::Error> for RouteErrorResponse {
    fn from(_: shell_serve::Error) -> Self {
        RouteErrorResponse(Status::InternalServerError)
    }
}

#[get("/<path..>")]
async fn _get(path: PathBuf, router: &State<ShellRouter>) -> Result<status::Custom<ReaderStream![impl AsyncRead]>, RouteErrorResponse> {
    let mut output = router.execute(&Method::Get, &path)?;
    let stdout = output.stdout()?;

    let status = output.wait().await?;

    // TODO trade-off of streaming is unknown Content-Length, do I care?
    // TODO how could the handler provide Content-Type? ReaderStream is octet-stream
    Ok(status::Custom(status, ReaderStream::one(stdout)))
}

#[put("/<path..>", data = "<data>")]
async fn _put(path: PathBuf, data: Data<'_>, router: &State<ShellRouter>) -> Result<status::Custom<ReaderStream![impl AsyncRead]>, RouteErrorResponse> {
    let mut stream = data.open(10.megabytes());

    let mut output = router.execute(&Method::Put, &path)?;

    output.write_stdin(&mut stream).await?;

    let stdout = output.stdout()?;
    let status = output.wait().await?;

    Ok(status::Custom(status, ReaderStream::one(stdout)))
}

#[delete("/<path..>")]
fn _delete(path: PathBuf) {
    println!("_delete {:?}", path);
}

#[launch]
fn rocket() -> _ {
    let router = ShellRouter::new(vec![
        "GET:/{path..}=echo Hello ${path}".parse::<Route>().unwrap(),
        "PUT:/{path..}=cat".parse::<Route>().unwrap()
    ]);

    rocket::build()
        .manage(router)
        .mount("/", routes![_get, _put, _delete])
}

// #[rocket::main]
// async fn main() {
//     // Recall that an uninspected `Error` will cause a pretty-printed panic,
//     // so rest assured errors do not go undetected when using `#[launch]`.
//     let _ = rocket().launch().await;
// }