use clap::builder::PossibleValue;
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(after_help = "Suggestions and bug reports are greatly appreciated:
https://github.com/zevtyardt/proxy.rs/issues")]
pub struct Cli {
    /// The maximum number of concurrent checks of proxies
    #[arg(long, default_value = "2000")]
    pub max_conn: usize,

    /// Time in seconds before giving up
    #[arg(short, long, default_value = "8")]
    pub timeout: usize,

    /// Logging level
    #[arg(long = "log", default_value = "warn", 
        value_parser([
            PossibleValue::new("debug"),
            PossibleValue::new("info"),
            PossibleValue::new("warn"),
            PossibleValue::new("error")
        ])
    )]
    pub log_level: String,

    /// Disable version checking
    #[arg(long)]
    pub skip_version_check: bool,

    #[command(subcommand)]
    pub sub: Commands,
}

#[derive(Subcommand, Debug, Clone)]
#[command(after_help = "Suggestions and bug reports are greatly appreciated:
https://github.com/zevtyardt/proxy.rs/issues")]
pub enum Commands {
    /// Find proxies without performing any checks
    Grab(GrabArgs),

    /// Find and check proxies
    Find(FindArgs),

    /// Run a local proxy server [BETA]
    Serve(ServeArgs),
}

#[derive(Args, Debug, Clone)]
#[command(after_help = "Suggestions and bug reports are greatly appreciated:
https://github.com/zevtyardt/proxy.rs/issues")]
pub struct GrabArgs {
    /// List of ISO country codes where should be located proxies
    #[arg(short, long, num_args(1..))]
    pub countries: Vec<String>,

    /// The maximum number of working proxies
    #[arg(short, long, default_value = "0")]
    pub limit: usize,

    /// Flag indicating in what format the results will be presented.
    #[arg(short, long,
        default_value = "default",
        value_parser([
            PossibleValue::new("default"),
            PossibleValue::new("text"),
            PossibleValue::new("json")
        ])
    )]
    pub format: String,

    /// Save found proxies to file. By default, output to console
    #[arg(short, long)]
    pub outfile: Option<std::path::PathBuf>,
}

#[derive(Args, Debug, Clone)]
#[command(after_help = "Suggestions and bug reports are greatly appreciated:
https://github.com/zevtyardt/proxy.rs/issues")]
pub struct FindArgs {
    /// Type(s) (protocols) that need to be check on support by proxy
    #[arg(long, required = true, num_args(1..),
        value_parser([
            PossibleValue::new("HTTP"),
            PossibleValue::new("HTTPS"),
            PossibleValue::new("SOCKS4"),
            PossibleValue::new("SOCKS5"),
            PossibleValue::new("CONNECT:80"),
            PossibleValue::new("CONNECT:25"),
        ]),
    )]
    pub types: Vec<String>,

    /// Path to the file with proxies. If specified, used instead of providers
    #[arg(long, num_args(1..))]
    pub files: Vec<std::path::PathBuf>,

    /// Level(s) of anonymity (for HTTP only). By default, any level
    #[arg(long, num_args(1..),
        value_parser([
            PossibleValue::new("Transparent"),
            PossibleValue::new("Anonymous"),
            PossibleValue::new("High")
        ])
    )]
    pub levels: Vec<String>,

    /// The maximum number of attempts to check a proxy
    #[arg(long, default_value = "1")]
    pub max_tries: usize,

    /// Flag indicating that the proxy must support cookies
    #[arg(long, default_value = "false")]
    pub support_cookies: bool,

    /// Flag indicating that the proxy must support referer
    #[arg(long, default_value = "false")]
    pub support_referer: bool,

    /// List of ISO country codes where should be located proxies
    #[arg(short, long, num_args(1..))]
    pub countries: Vec<String>,

    /// The maximum number of working proxies
    #[arg(short, long, default_value = "0")]
    pub limit: usize,

    /// Flag indicating in what format the results will be presented.
    #[arg(short, long,
        default_value = "default",
        value_parser([
            PossibleValue::new("default"),
            PossibleValue::new("text"),
            PossibleValue::new("json")
        ])
    )]
    pub format: String,

    /// Save found proxies to file. By default, output to console
    #[arg(short, long)]
    pub outfile: Option<std::path::PathBuf>,
}

#[derive(Args, Debug, Clone)]
#[command(after_help = "Suggestions and bug reports are greatly appreciated:
https://github.com/zevtyardt/proxy.rs/issues")]
pub struct ServeArgs {
    /// Host of local proxy swrver
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port of local proxy swrver
    #[arg(long, default_value = "8080")]
    pub port: u16,

    /// Type(s) (protocols) that need to be check on support by proxy
    #[arg(long, required = true, num_args(1..),
        value_parser([
            PossibleValue::new("HTTP"),
            PossibleValue::new("HTTPS"),
            PossibleValue::new("SOCKS4"),
            PossibleValue::new("SOCKS5"),
            PossibleValue::new("CONNECT:80"),
        ]),
    )]
    pub types: Vec<String>,

    /// Path to the file with proxies. If specified, used instead of providers
    #[arg(long, num_args(1..))]
    pub files: Vec<std::path::PathBuf>,

    /// Level(s) of anonymity (for HTTP only). By default, any level
    #[arg(long, num_args(1..),
        value_parser([
            PossibleValue::new("Transparent"),
            PossibleValue::new("Anonymous"),
            PossibleValue::new("High")
        ])
    )]
    pub levels: Vec<String>,

    /// The maximum number of attempts to check a proxy
    #[arg(long, default_value = "1")]
    pub max_tries: usize,

    /// List of ISO country codes where should be located proxies
    #[arg(short, long, num_args(1..))]
    pub countries: Vec<String>,
}
