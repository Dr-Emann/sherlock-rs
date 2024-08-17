use clap::Parser;
use color_eyre::Result;
use sherlock_rs::{
    default_data::get_default_data, output::save_results, sherlock::check_username,
    sherlock_target_manifest::SherlockTargetManifest,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(name = "sherlock")]
#[command(author = "Johannes Naylor <jonaylor89@gmail.com>")]
#[command(version = "1.0")]
#[command(about = "Hunt down social media accounts by username", long_about = None)]
struct Cli {
    /// One or more usernames to check with social networks. Check similar usernames using {?} (replace to '_', '-', '.').
    #[clap(name = "usernames", required = true)]
    usernames: Vec<String>,

    /// Display extra debugging information and metrics.
    #[clap(short, long, alias = "debug")]
    verbose: bool,

    /// The output file to save the results to.
    #[clap(short, long = "output")]
    output_file: Option<String>,

    /// If using single username, the output of the result will be saved to this file.
    #[clap(short = 'f', long, alias = "output-folder")]
    output_folder: Option<String>,

    /// Make requests over Tor; increases runtime; requires Tor to be installed and in system path.
    #[clap(long, alias = "tor")]
    tor: bool,

    /// Make requests over Tor with new Tor circuit after each request; increases runtime; requires Tor to be installed and in system path.
    #[clap(long, alias = "unique-tor")]
    unique_tor: bool,

    /// Create Comma-Separated Values (CSV) File.
    #[clap(short, long, alias = "csv")]
    csv: bool,

    /// Create the standard file for the modern Microsoft Excel spreadsheet (xlsx).
    #[clap(long)]
    xlsx: bool,

    /// Limit analysis to just the listed sites. Add multiple options to specify more than one site.
    #[clap(short, long)]
    site_list: Vec<String>,

    // Make requests over a proxy. e.g. socks5://127.0.0.1:1080
    #[clap(short, long, alias = "proxy")]
    proxy: Option<String>,

    /// Dump the HTTP request to stdout for targeted debugging.
    #[clap(short, long)]
    dump_response: bool,

    /// Load data from a JSON file or an online, valid, JSON file.
    #[clap(short, long = "json")]
    json_file: Option<String>,

    /// Time (in seconds) to wait for response to requests.
    #[clap(short, long, alias = "timeout", default_value_t = 60)]
    timeout: u64,

    /// Output sites where the username was not found.
    #[clap(long, alias = "print-all")]
    print_all: bool,

    /// Output sites where the username was found.
    #[clap(long, alias = "print-found", default_value_t = true)]
    print_found: bool,

    /// Don't color terminal output.
    #[clap(short, long, alias = "no-color")]
    no_color: bool,

    /// Browse to all results on default browser.
    #[clap(short, long, alias = "browse")]
    browse: bool,

    /// Force the use of the local data.json file.
    #[clap(short, long, alias = "local")]
    local: bool,

    /// Include checking of NSFW sites from default list.
    #[clap(long, alias = "nsfw")]
    nsfw: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    let json_data = get_default_data();
    // let json_data = include_str!("../resources/data.json");
    let deserializer = &mut serde_json::Deserializer::from_str(&json_data);
    let initial_data: SherlockTargetManifest = serde_path_to_error::deserialize(deserializer)
        .map_err(|err| {
            println!("[!!!] error path [{}]", err.path().to_string());
            err
        })?;

    for username in cli.usernames {
        let results = check_username(username, initial_data.targets.clone()).await?;
        save_results(results)?;
    }

    Ok(())
}
