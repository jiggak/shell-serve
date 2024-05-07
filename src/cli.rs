pub use clap::Parser;
use serde::{Deserialize, Deserializer};
use shell_serve::route::Route;
use std::{fs, io, net::IpAddr, path::{Path, PathBuf}};


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Load options and routes from `.toml` file
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    #[arg(short, long, default_value = "127.0.0.1")]
    pub listen: IpAddr,

    #[arg(short, long, default_value = "8000")]
    pub port: u16,

    #[arg(required_unless_present = "file", num_args = 1..)]
    pub routes: Vec<Route>
}

impl Cli {
    pub fn load_config<P>(mut self, file_path: P) -> Result<Self, ConfigError>
        where P: AsRef<Path>
    {
        let toml_src = fs::read_to_string(file_path)?;
        let config: ConfigFile = toml::from_str(&toml_src)?;

        if let Some(listen) = config.listen {
            self.listen = listen;
        }

        if let Some(port) = config.port {
            self.port = port;
        }

        if let Some(routes) = config.routes {
            self.routes.extend(routes);
        }

        Ok(self)
    }
}

#[derive(Deserialize)]
struct ConfigFile {
    listen: Option<IpAddr>,
    port: Option<u16>,
    #[serde(default, deserialize_with = "config_file_routes")]
    routes: Option<Vec<Route>>
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ConfigRoute {
    String(String),
    Object { method: String, path: String, handler: String }
}

impl ToString for ConfigRoute {
    fn to_string(&self) -> String {
        match self {
            ConfigRoute::String(s) => s.to_owned(),
            ConfigRoute::Object { method, path, handler } =>
                format!("{method}:{path} {handler}")
        }
    }
}

fn config_file_routes<'de, D>(deserializer: D) -> Result<Option<Vec<Route>>, D::Error>
    where D: Deserializer<'de>
{
    let routes: Vec<ConfigRoute> = Deserialize::deserialize(deserializer)?;
    let routes = routes.iter()
        .map(|s| s.to_string().parse::<Route>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(serde::de::Error::custom)?;

    Ok(Some(routes))
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file")]
    FileIoError(#[from] io::Error),

    #[error("Failed to parse config file")]
    TomlParseError(#[from] toml::de::Error),

    #[error("Invalid 'listen' address")]
    InvalidIpAddr(#[from] std::net::AddrParseError),

    #[error("Route parse error")]
    RouteParse(#[from] shell_serve::Error)
}