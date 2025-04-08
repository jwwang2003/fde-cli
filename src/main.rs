mod constants;
mod cli;
mod vlfd;
mod ports;
mod helper;
mod manager;

use anyhow::Result;
use owo_colors::OwoColorize;

fn main() -> Result<()> {
    println!("{}", constants::GREETING);
    println!("Brought to you by {}", constants::AUTHOR.green());
    
    cli::run_cli()?;
    
    Ok(())
}