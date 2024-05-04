pub use clap::Parser;
use shell_serve::route::Route;
use std::net::IpAddr;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value = "127.0.0.1")]
    pub listen: IpAddr,

    #[arg(short, long, default_value = "8000")]
    pub port: u16,

    #[arg(required = true, num_args = 1..)]
    pub routes: Vec<Route>
}