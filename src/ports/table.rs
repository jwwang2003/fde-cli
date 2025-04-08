/**
 * Filename: table.rs
 * Desciprtion: A helper class that outputs a "printable" table for the Tabled library
 */

use tabled::Tabled;

use super::{IOType, IOPort};

#[derive(Tabled)]
pub struct IOPortsTable<'a> {
    io_type: &'a IOType,
    port_name: &'a str,
    data: u64,
}

impl<'a> IOPortsTable<'a> {
  pub fn from_io(io_ports: &'a Vec<IOPort>) -> Vec<Self> {
    let mut ports: Vec<Self> = Vec::new();
    for port in io_ports.iter() {
      if let IOType::DC = port.io_type { continue; }
      ports.push(Self { io_type: &port.io_type, port_name: &port.io_name, data: port.data})
    }
    return ports;
  }
}