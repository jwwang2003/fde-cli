use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json;
use anyhow::Result;

// Define a struct that matches the JSON structure.
#[derive(Serialize, Deserialize, Clone)]
pub struct PortMappings {
    pub input: HashMap<String, i32>,
    pub output: HashMap<String, i32>,
}

// Helper method to parse the JSON string into input_ports and output_ports.
pub fn parse_ports(json_str: &str) -> Result<(HashMap<String, i32>, HashMap<String, i32>), Box<dyn std::error::Error>> {
    // Deserialize the JSON into our PortMappings struct.
    let mappings: PortMappings = serde_json::from_str(json_str)?;
    
    // Separate the input and output ports.
    Ok((mappings.input, mappings.output))
}