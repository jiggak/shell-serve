#[macro_use] extern crate rocket;

use rocket::{http::Status, State};
use shell_serve::route::{Method, Route, RouteOutput};
use shell_serve::router::{RouterError, ShellRouter};
use std::path::PathBuf;


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

#[put("/<path..>")]
fn _put(path: PathBuf) {
    println!("_put {:?}", path);
}

#[delete("/<path..>")]
fn _delete(path: PathBuf) {
    println!("_delete {:?}", path);
}

#[launch]
fn rocket() -> _ {
    let router = ShellRouter::new(vec![
        "GET:/{path..}=echo Hello ${path}".parse::<Route>().unwrap()
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