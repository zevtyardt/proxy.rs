use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(about)]
pub struct Cli {
    ///The maximum number of concurrent checks of proxies
    #[arg(long, default_value = "200")]
    pub max_conn: usize,

    ///The maximum number of attempts to check a proxy
    #[arg(long, default_value = "1")]
    pub max_tries: usize,

    ///Time in seconds before giving up
    #[arg(short, long, default_value = "8")]
    pub timeout: usize,

    ///The maximum number of working proxies
    #[arg(short, long)]
    pub limit: Option<usize>,
}
