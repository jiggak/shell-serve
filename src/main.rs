use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use shell_serve::{route::Route, router::ShellRouter};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let addr: SocketAddr = ([127, 0, 0, 1], 8000).into();

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);

    let router = ShellRouter::new(vec![
        "GET:/{path..}?{query..} ./foo.sh ${path} ${query}".parse::<Route>().unwrap(),
        "PUT:/{path..} cat".parse::<Route>().unwrap()
    ]);

    loop {
        let (tcp, _) = listener.accept().await?;
        let io = TokioIo::new(tcp);

        let service = router.clone();

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}