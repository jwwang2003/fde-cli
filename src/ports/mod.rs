/**
 * Filename: mod.rs
 * Desciprtion: The entry folder that contains all the main components for "port" functionality
 */

use std::collections::HashMap;
use anyhow::Result;
use regex::Regex;
use tabled::Tabled;
use std::fmt;

mod parse;
pub mod table;

#[derive(Debug, Clone)]
pub struct ConstraintPort {
    pub name: String,
    pub port_name: String,
}

#[derive(Debug, Clone, Tabled)]
pub enum IOType { INPUT, OUTPUT, DC }   // DC -> Don't Care
impl fmt::Display for IOType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IOType::DC => write!(f, "DC"),
            IOType::INPUT => write!(f, "INPUT"),
            IOType::OUTPUT => write!(f, "OUTPUT"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Port {
    name: String,
    pin_name: String,
    pin_index: i32,
    value: bool
}

/// Given a port constraint (a port entity read from the contraints .xml file),
/// contruct a Port wrapper around it and return it
pub fn new_port(constraint_p: ConstraintPort, port_mapping: parse::PortMappings) -> Port {
    // Look for the pin in the input lookup first.
    let pin_index = if let Some(&index) = port_mapping.input.get(&constraint_p.port_name) {
        index
    } else if let Some(&index) = port_mapping.output.get(&constraint_p.port_name) {
        // If not found in inputs, try the output lookup.
        index - 1
    } else {
        // Fallback
        -1
    };

    return Port {
        name: constraint_p.name,
        pin_name: constraint_p.port_name,
        pin_index: pin_index,
        value: false
    }
}

#[derive(Debug, Clone)]
pub struct IOPort {
    pub io_type: IOType,
    pub io_name: String,
    pub ports: Vec<Port>,
    pub data: u64
}

impl IOPort {
    pub fn new(io_type: IOType, io_name: String, ports: Vec<Port>) -> Self {
        Self { io_type, io_name , ports, data: 0u64 }
    }

    /// Returns a u64 decimal of the data represented by the port(s)
    pub fn get_value(&self) -> u64 {
        return self.data;
    }

    /// Change the value represented by the ports (update/mutate),
    /// the boolean value of each individual port will be updated too.
    pub fn change_value(&mut self, new_data: u64) {
        self.data = 0u64;
        let base = self.ports[0].pin_index;

        for port in self.ports.iter_mut() {
            if port.pin_index == -1 { continue; }
            let pin = port.pin_index - base;
            // Update port value
            port.value = ((new_data >> (pin)) & 0x1) != 0;
            self.data |= (port.value as u64) << pin;
        }
    }

    /// Updates the value of the port(s) and the data it represents from
    /// the snapshot of one clock cycle (u64).
    pub fn update(&mut self, bitstream: u64) {
        self.data = 0u64;
        let base = self.ports[0].pin_index;

        for port in self.ports.iter_mut() {
            if port.pin_index == -1 { continue; }
            let pin = port.pin_index - base;
            port.value = ((bitstream >> (port.pin_index)) & 0x1) != 0;
            self.data |= (port.value as u64) << pin;
        }
    }

    /// Returns a 64-bit uint that contains the current state (or data) of this IO
    /// depending on the IO pins (unrelated pins would be set to 0).
    /// All the return values of the IO ports can then be ORed together to produce the
    /// final 64-bit uint to be sent to FDE's FIFO.
    pub fn get_write(&self) -> u64 {
        let mut temp: u64 = 0u64;
        // Do not handle the offset here

        for port in self.ports.iter() {
            if port.pin_index == -1 { continue; }
            let pin = port.pin_index;
            temp |= (port.value as u64 & 0x1) << pin;
        }
        return temp;
    }
}

/// Groups ports that match the 1D array pattern into a vector of IO_Port.
/// The key for each IO_Port is the base array name. The `input_lookup` and
/// `output_lookup` maps are used to decide the IO_Type for the grouped port.
pub fn group_ports(
    ports: &Vec<Port>,
    port_mapping: parse::PortMappings
) -> Vec<IOPort> {
    let (input_lookup, output_lookup) = (port_mapping.input, port_mapping.output);
    
    // Regex to match strings like "arr[0]", "data[15]", etc.
    let array_re = Regex::new(r"^(\w+)\[(\d+)\]$").unwrap();

    // Use a temporary HashMap to group ports by base name.
    let mut groups: HashMap<String, Vec<Port>> = HashMap::new();

    for port in ports {
        if let Some(caps) = array_re.captures(&port.name) {
            let base_name = caps.get(1).unwrap().as_str().to_string();
            // Insert the port (cloned) into the group corresponding to the base name.
            groups.entry(base_name).or_insert_with(Vec::new).push(port.clone());
        } else {
            groups.entry(port.name.to_string()).or_insert_with(Vec::new).push(port.clone())
        }
    }

    // Now create IO_Port instances from the grouped ports.
    let mut io_ports = Vec::new();
    for (base_name, group) in groups {
        // Determine the IO_Type using the provided lookup maps.
        // Here, we check the input_lookup first; if not present, we check output_lookup.
        let pin_name: &str = &group.first().unwrap().pin_name;
        let io_type = if input_lookup.contains_key(pin_name) {
            IOType::INPUT
        } else if output_lookup.contains_key(pin_name) {
            IOType::OUTPUT
        } else {
            // Fallback
            IOType::DC
        };

        io_ports.push(IOPort::new(io_type, base_name, group));
    }

    io_ports
}

/// FDE board input/output pins are stored in a folder relative to the root of the project: fde/VERICOMM_MAP.json
pub fn fde_parse_ports() -> Result<parse::PortMappings, Box<dyn std::error::Error>> {
    let json_file_path = "fde/VERICOMM_MAP.json";

    // Read the JSON file into a string.
    let json_data = std::fs::read_to_string(json_file_path)
        .expect(&format!("Failed to read JSON file at {}", json_file_path));

    match parse::parse_ports(&json_data) {
        Ok((input, output)) => { return Ok(parse::PortMappings {input, output} ); }
        Err(e) => {
            eprintln!("Error parsing JSON: {}", e);
            return Err(e);
        }
    }
}

/// Helper function that converts a slice of 4 u16 elements into a u64 
/// by shifting and combining them into a u64 (input size must be a factor of 4)
pub fn u16_4_to_u64(rx_buffer: &[u16]) -> u64 {
    // Ensure the slice has exactly 4 elements
    assert_eq!(rx_buffer.len(), 4);
    let mut result = 0u64;
    // Shift and combine the u16 elements to form a u64
    for (i, &num) in rx_buffer.iter().enumerate() {
        result |= (num as u64) << (16 * (3 - i)); // Shift each u16 value by 16 * (3 - i)
    }

    result
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use super::super::helper::constraints::ConstraintsReader;

    #[test]
    fn test_ports() {
        let result = fde_parse_ports();
        assert!(result.is_ok(), "Expected valid JSON to be parsed successfully");
        let port_mappings = result.unwrap();
        println!("{:?}", port_mappings.input);
        println!("{:?}", port_mappings.output);
    }

    #[test]
    fn test_ports_from_constraints() {
        let port_mappings = fde_parse_ports().unwrap();

        // Read constraints
        let xml_data_path = "recipes/name_display/name_display_cons.xml";

        let mut reader = ConstraintsReader::new(xml_data_path);
        let _ = reader.read();
        let constraints = reader.get_ports();

        for constraint in constraints.iter() {
            let new_port = new_port(constraint.clone(), port_mappings.clone());
            println!("{:?}", new_port);
        }
    }

    #[test]
    fn test_grouping_groups() {
        // Get port mappings
        let port_mappings = fde_parse_ports().unwrap();

        // Read constraints
        let xml_data_path = "recipes/name_display/name_display_cons.xml";

        let mut reader = ConstraintsReader::new(xml_data_path);
        let _ = reader.read();
        let constraints = reader.get_ports();

        let mut port_vec: Vec<Port> = Vec::new();

        for constraint in constraints.iter() {
            let new_port = new_port(constraint.clone(), port_mappings.clone());
            println!("{:?}", new_port);
            port_vec.push(new_port);
        }

        let grouped = group_ports(&port_vec, port_mappings);

        for group in grouped {
            println!("{:#?} {}", group.io_type, group.io_name);
            println!("{:#?}", group.ports);
        }
    }

    #[test]
    fn test_group_data_update() {
        let bit_data: u64 = 0x003ffffffffff628;
        // 0000000000111111 1111111111111111 1111111111111111 1111011000101000
        
        let port_mappings = fde_parse_ports().unwrap();

        // Read constraints
        let xml_data_path = "recipes/name_display/name_display_cons.xml";

        let mut reader = ConstraintsReader::new(xml_data_path);
        let _ = reader.read();
        let constraints = reader.get_ports();

        let mut port_vec: Vec<Port> = Vec::new();

        for constraint in constraints.iter() {
            let new_port = new_port(constraint.clone(), port_mappings.clone());
            println!("{:?}", new_port);
            port_vec.push(new_port);
        }

        let grouped = group_ports(&port_vec, port_mappings);

        for group in grouped.iter().clone() {
            println!("{:#?} {}", group.io_type, group.io_name);
            println!("{:#?}", group.ports);
        }

        let mut e: IOPort = grouped.iter().find(|&e| e.io_name == "lcd_db").unwrap().clone();
        e.update(bit_data);
        println!("{:#?}", e);
        println!("{:04x}", e.data);

        println!("Write data: 0x{:016x}", e.get_write());

        assert_eq!(e.data, 0x28);
    }

}