/**
 * Filename: parse.rs
 * Desciprtion: Helper functions for parsing the constraints (.xml) file
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json;

// Define a struct that matches the JSON structure.
#[derive(Serialize, Deserialize, Clone)]
pub struct PortMappings {
    pub input: HashMap<String, i32>,
    pub output: HashMap<String, i32>,
}

// Helper method to parse the JSON string into port mappings.
pub fn parse_ports(json_str: &str) -> Result<PortMappings, Box<dyn std::error::Error>> {
    // Deserialize the JSON into our PortMappings struct.
    let mappings: PortMappings = serde_json::from_str(json_str)?;

    Ok(mappings)
}
