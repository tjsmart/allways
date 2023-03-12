use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;

use allways::do_it_allways;

fn main() -> Result<()> {
    let args = Args::parse();

    check_files(&args.paths)?;

    let mut rtc = 0;
    for file in &args.paths {
        let src = std::fs::read_to_string(file)?;
        if let Some(new_src) = do_it_allways(&src)? {
            if src != new_src {
                println!("Updating __all__ statement in {}", file.display());
                std::fs::write(file, new_src)?;
                rtc |= 1;
            }
        }
    }

    std::process::exit(rtc);
}

fn check_files(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        if !path.exists() {
            Err(anyhow!("Path {:?} does not exist!", path))?;
        }
    }
    Ok(())
}

/// Automatically update `__all__` statements in python libraries.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Any number of python files.
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,
}
