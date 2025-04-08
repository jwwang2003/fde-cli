/**
 * Filename: bitstream.rs
 * Description: Handles the reading & parsing of the bitstream (.bit) file
 */

use std::fs::File;
use std::io::BufRead;
use std::vec::Vec;
use owo_colors::OwoColorize;
 
// 'a is a lifetime parameter
pub struct ProgramDataReader {
     bitfile: String,
     // tells the Rust compiler that the reference stored in bitfile must be valid for
     // at least as long as the lifetime of 'a
     program_data: Vec<u16>,
 }
 
impl ProgramDataReader {
     // Constructor to initialize with the file path.
     pub fn new(bitfile: &str) -> Self {
         ProgramDataReader {
             bitfile: bitfile.to_string(),
             program_data: Vec::new(),
         }
     }
 
     // Check if the file is readable and process the file content.
     pub fn read(&mut self) -> Result<(), String> {
         let file = File::open(&self.bitfile)
           .map_err(|_e| "failed to open bitstream (.bit) file".to_string())?;
 
         let lines = std::io::BufReader::new(file).lines();
         let mut program_data = Vec::with_capacity(lines.size_hint().0 * 2);
 
         for (line_n, line) in lines.enumerate() {
             let line = line
               .map_err(|_err| "failed to read bitstream file".to_string())?;
 
             let line = line.trim();
             if line.is_empty() {
                 continue;
             }
 
             let mut data = 0u16;
 
             for (col_n, c) in line.as_bytes().iter().enumerate() {
                 match *c {
                     b'_' => {
                         program_data.push(data);
                         data = 0;
                         continue;
                     }
                     b' ' | b'\t' => {
                         break;
                     }
                     _ => {}
                 }
 
                 let remapped = self.char_remap(c);
                 if remapped.is_none() {
                     // return Err("invalid char in bitfile".to_string());
                     // Provide some useful debugging information
                     return Err(format!(
                       "invalid char '{}' at {},{} in .bit file",
                       *c as char, line_n + 1, col_n + 1
                   ));
                 }
 
                 data = (data << 4) | (remapped.unwrap() as u16);
             }
             program_data.push(data);
         }
 
         self.program_data = program_data;
         Ok(())
     }
     // [C++ Implementation Reference] ProgramVLFD.cpp 92,38
 
     // Helper function to remap characters
     fn char_remap(&self, c: &u8) -> Option<u8> {
       let result = match c {
           0x30..=0x39 => c - 0x30,
           0x41..=0x46 => c - 0x37,
           0x61..=0x66 => c - 0x57,
           _ => return None,
       };
   
       Some(result)
     }
     // [C++ Implementation Reference] ProgramVLFD.cpp, 184，5
     // while (!fi.eof())
     // {
     // 	fi.read(&ctemp, 1);
     // 	if (ctemp >= 0x30 && ctemp <= 0x39)
     // 		ctemp -= 0x30;
     // 	else if (ctemp >= 0x61 && ctemp <= 0x66)
     // 		ctemp -= 0x57;
     // 	else if (ctemp >= 0x41 && ctemp <= 0x46)
     // 		ctemp -= 0x37;
     // 	else
     // 		break;
     // 	th = (th << 4) | ctemp;
     // }
   
 
     // Getter to retrieve the program data.
     pub fn get_program_data(&self) -> &Vec<u16> {
         &self.program_data
     }
     
     // pub fn print_program_data(&self, whole: Option<bool>) {
     //   let print_all = whole.unwrap_or(false);
       
     //   if (print_all) {
     //     println!("Program data: {:?}", &self.program_data);
     //   } else {
     //     println!("Program data: {:?}", &self.program_data.get(..20).unwrap());
     //   }
     // }
 
     pub fn preview_prorgam_data(&self) {
       println!("{}:\n\t{:?}",
         "Preview bitstream".green(),
         &self.program_data.get(..20).unwrap());
     }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_valid_bitstream() {
        let file_path = "recipes/name_display/name_display_dc_bit.bit";
        // Initialize the ProgramDataReader with the test file.
        let mut reader = ProgramDataReader::new(file_path);

        // Call the read method and check that it succeeds.
        let result = reader.read();
        assert!(result.is_ok(), "Expected reading to succeed, got error: {:?}", result.err());

        // Verify the resulting program data.
        let program_data = reader.preview_prorgam_data();
        println!("{:#?}", program_data);
    }
}