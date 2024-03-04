#[macro_use] extern crate rocket;

use rocket::{data::{Data, ToByteUnit}, http::Status, State};
use rocket::response::stream::ReaderStream;
use rocket::tokio::io::AsyncRead;
use shell_serve::route::{Method, Route};
use shell_serve::router::ShellRouter;
use std::path::PathBuf;

/*
// TODO Consider using rocket responders for RouteOutput and RouteError
fn route_output_response(result: Result<RouteOutput, RouterError>) -> (Status, Vec<u8>) {
    let output = match result {
        Ok(output) => output,
        Err(e) => return match e {
            RouterError::RouteNotFound => (Status::NotFound, vec![]),
            RouterError::RouteSpawnFailed(_) => (Status::InternalServerError, vec![])
        }
    };

    // TODO how to return 400 errors when process exit status limit is 255?
    let status = if output.status_ok() {
        Status::Ok
    } else {
        Status::InternalServerError
    };

    let body = output.stdout();

    // TODO can I somehow stream command output without buffering?
    (status, body.to_vec())
}

#[get("/<path..>")]
fn _get(path: PathBuf, router: &State<ShellRouter>) -> (Status, Vec<u8>) {
    let result = router.execute(&Method::Get, &path);
    route_output_response(result)
}
 */

#[get("/<path..>")]
async fn _get(path: PathBuf, router: &State<ShellRouter>) -> (Status, ReaderStream![impl AsyncRead]) {
    let mut output = router.execute(&Method::Get, &path).unwrap();
    let stdout = output.stdout().unwrap();

    let success = output.wait().await.unwrap();
    // TODO how to return 400 errors when process exit status limit is 255?
    let status = if success {
        Status::Ok
    } else {
        Status::InternalServerError
    };

    // TODO trade-off of streaming is unknown Content-Length, do I care?
    // TODO how could the handler provide Content-Type? ReaderStream is octet-stream
    (status, ReaderStream::one(stdout))
}

#[put("/<path..>", data = "<data>")]
async fn _put(path: PathBuf, data: Data<'_>, router: &State<ShellRouter>) -> (Status, ReaderStream![impl AsyncRead]) {
    let mut stream = data.open(10.megabytes());

    let mut output = router.execute(&Method::Put, &path).unwrap();

    output.write_stdin(&mut stream).await.unwrap();

    let stdout = output.stdout().unwrap();
    let success = output.wait().await.unwrap();
    let status = if success {
        Status::Ok
    } else {
        Status::InternalServerError
    };

    (status, ReaderStream::one(stdout))
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