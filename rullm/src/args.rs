use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    // This should be a vector eventually
    pub conf_file: Option<PathBuf>,
}

