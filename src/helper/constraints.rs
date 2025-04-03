/**
 * Filename: constraints.rs
 * Desciprtion: Handles the reading and parsing of the constriant .xml file
 * Checking if the required ports are matched, and matched correctly
 */

use std::fs::File;
use std::vec::Vec;
use xml::reader::{EventReader, XmlEvent};
use owo_colors::OwoColorize;

use super::super::ports::Constraint_Port;

/// Note that ports contains a variable_name -> port_name mapping
pub struct ConstraintsReader {
    constraintsfile: String,
    ports: Vec<Constraint_Port>
}
 
impl ConstraintsReader {
     pub fn new(constraintsfile: String) -> Self {
         ConstraintsReader { constraintsfile, ports: Vec::new() }
     }
 
     pub fn read(&mut self) -> Result<(), String> {
         let file = File::open(&self.constraintsfile)
           .map_err(|_e| "failed to open constraints (.xml) file".to_string())?;
 
         let parser = EventReader::new(file);
 
         // Iterate through the XML events
         for e in parser {
             match e {
                 Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                     if name.local_name == "port" {
                         let mut port_name = String::new();
                         let mut position = String::new();
 
                         // Extract the "name" and "position" attributes
                         for attr in attributes {
                             if attr.name.local_name == "name" {
                                 port_name = attr.value;
                             }
                             else if attr.name.local_name == "position" {
                                 position = attr.value;
                             }
                         }
 
                         // Store the port information in the vector
                         self.ports.push(Constraint_Port{ name: port_name, port_name: position});
                     }
                 }
                 // Ok(XmlEvent::EndElement { name }) => {
                 //     // Handle closing tags (not needed in this case)
                 // }
                 Err(e) => {
                     eprintln!("Error: {e}");
                     break;
                 }
                 _ => {}
             }
         }
 
         Ok(())
     }
 
     // Specialized purposes, this XML parser is for port -> position mappings
 
     /// Getter method for port constraints
     pub fn get_ports(&self) -> &Vec<Constraint_Port> {
         &self.ports
     }
     
     /// Print port constraints
     pub fn print_ports(&self) {
         println!("{} (port name, position)", "Constraints".green());
         for port in &self.ports {
             println!("\t{}, {}", port.name, port.port_name);
         }
     }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_constraints() {
        let xml_data_path = "projects/name_display/name_display_cons.xml";

        let mut reader = ConstraintsReader::new(xml_data_path.to_string());
        let result = reader.read();
        assert!(result.is_ok(), "Expected XML file to be read successfully");
        
        reader.print_ports();
    }
}