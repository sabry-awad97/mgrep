use std::path::PathBuf;

use structopt::StructOpt;

/// Represents the command-line arguments for the program.
#[derive(StructOpt)]
pub struct Cli {
    /// The search term
    pub search_term: String,

    /// The directory to search in
    #[structopt(parse(from_os_str))]
    pub search_dir: PathBuf,
}
