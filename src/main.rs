mod constants;
mod cli;                // Main CLI logic
mod ports;              // Encoding & decoding SMIMS VLFD IO port data
mod helper;             // Helper functions
mod manager;            // Project/recipe manager

use anyhow::Result;
use owo_colors::OwoColorize;

fn main() -> Result<()> {
    println!("{}", constants::GREETING);
    println!("Brought to you by {}", constants::AUTHOR.green());
    
    cli::run_cli()?;
    
    Ok(())
}
