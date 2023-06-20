use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Cli {
    /// The search term
    pub search_term: String,

    /// The directory to search in
    #[structopt(parse(from_os_str), default_value = ".")]
    pub search_dir: PathBuf,
}
