#[macro_use] extern crate rocket;

use rocket::State;
use shell_serve::{route::{Method, Route},router::ShellRouter};
use std::path::PathBuf;


#[get("/<path..>")]
fn _get(path: PathBuf, router: &State<ShellRouter>) -> String {
    // format!("Hello, world! {:?} {:?}", path, route)
    router.execute(&Method::Get, &path)
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