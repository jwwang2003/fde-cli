mod constants;
mod cli;                // Main CLI logic
mod vlfd;               // VeriComm driver
mod ports;              // Encoding & decoding SMIMS VLFD IO port data
mod helper;             // Helper functions
mod utilities;          // Major features will be implemented here
mod manager;            // Project/recipe manager
mod file_parser;        // various ways of reading data from a file & parsing it into a stream of bits

use anyhow::Result;
use owo_colors::OwoColorize;

fn main() -> Result<()> {
    println!("{}", constants::GREETING);
    println!("Brought to you by {}", constants::AUTHOR.green());
    
    cli::run_cli()?;
    
    Ok(())
}