use clap::builder::PossibleValue;
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
pub struct Cli {
    /// The maximum number of concurrent checks of proxies
    #[arg(long, default_value = "50")]
    pub max_conn: usize,

    /// The maximum number of attempts to check a proxy
    #[arg(long, default_value = "1")]
    pub max_tries: usize,

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

    #[command(subcommand)]
    pub sub: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Find proxies without a checks
    Grab(GrabArgs),

    /// Find and check proxies
    Find(FindArgs),
}

#[derive(Args, Debug, Clone)]
pub struct GrabArgs {
    /// List of ISO country codes where should be located proxies
    #[arg(short, long, num_args(1..))]
    pub countries: Vec<String>,

    /// The maximum number of working proxies
    #[arg(short, long)]
    pub limit: Option<usize>,

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
}
#[derive(Args, Debug, Clone)]
pub struct FindArgs {
    /// Type(s) (protocols) that need to be check on support by proxy
    #[arg(long, required = true, num_args(1..),
        value_parser([
            PossibleValue::new("HTTP"),
            PossibleValue::new("HTTPS"),
            PossibleValue::new("SOCKS4"),
        ]),
    )]
    pub types: Vec<String>,

    /// Level(s) of anonymity (for HTTP only). By default, any level
    #[arg(long, num_args(1..),
        value_parser([
            PossibleValue::new("Transparent"),
            PossibleValue::new("Anonymous"),
            PossibleValue::new("High")
        ])
    )]
    pub levels: Vec<String>,

    /// List of ISO country codes where should be located proxies
    #[arg(short, long, num_args(1..))]
    pub countries: Vec<String>,

    /// The maximum number of working proxies
    #[arg(short, long)]
    pub limit: Option<usize>,

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
}
