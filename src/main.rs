mod cli;

use cli::{Cli, Parser};
use hyper::{body, server::conn::http1, service::service_fn, Request};
use hyper_util::rt::TokioIo;
use shell_serve::router::ShellRouter;
use std::net::SocketAddr;
use tokio::net::TcpListener;


#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let addr = SocketAddr::new(cli.listen, cli.port);

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);

    let router = ShellRouter::new(cli.routes);

    loop {
        let (tcp, _) = listener.accept().await?;
        let io = TokioIo::new(tcp);

        let router = router.clone();

        let service = service_fn(move |req: Request<body::Incoming>| {
            let router = router.clone();
            async move { router.call(req).await }
        });

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}