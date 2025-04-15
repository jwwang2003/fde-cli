use std::fs::File;
use std::io::{self, BufRead};
use anyhow::Result;
use std::io::Read;

fn read_hex_data_from_text_file<P: AsRef<std::path::Path>>(file_path: P) -> Result<Vec<String>> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file); // This is the BufReader

    let mut values = Vec::new();
    
    // Iterate over lines in the file
    for line in reader.lines() {
        let line = line?; // Unwrap the Result
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue; // Ignore empty lines or comments
        }
        values.push(line.trim().to_string());
    }

    Ok(values)
}

fn read_binary_data_from_file<P: AsRef<std::path::Path>>(file_path: P) -> Result<Vec<u8>> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_parser_with_valid_input() {
        let mut temp_file = NamedTempFile::new().unwrap();
        
        // New test content (including comments)
        let content = r#"
0x0600
0x0000
0x0000
0x0000

0x0000
0x0000
0x0000
0x0000

0x0200
0x0000
0x0000
0x0000

# Pause for one clock cycle
0x0400
0x0000
0x0000
0x0000

# Send '1' to FIFO
0x0C01
0x0000
0x0000
0x0000

# Send '2' to FIFO
0x0C02
0x0000
0x0000
0x0000

# Send '3' to FIFO
0x0C03
0x0000
0x0000
0x0000

# Pause for two clock cycles
0x0400
0x0000
0x0000
0x0000

0x0400
0x0000
0x0000
0x0000

# Read '1' from RX FIFO
0x1400
0x0000
0x0000
0x0000

0x1400
0x0000
0x0000
0x0000

# Read '2' from RX FIFO
0x1400
0x0000
0x0000
0x0000

0x1400
0x0000
0x0000
0x0000

# Read '3' from RX FIFO
0x1400
0x0000
0x0000
0x0000

0x1400
0x0000
0x0000
0x0000

# Pause for one clock cycle
0x0400
0x0000
0x0000
0x0000"#;
        
        write!(temp_file, "{}", content).unwrap();

        let result = read_hex_data_from_text_file(temp_file.path()).unwrap();

        let expected = vec![
            "0x0600".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),

            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x0200".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x0400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x0C01".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x0C02".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x0C03".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x0400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x0400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x1400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x1400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x1400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x1400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x1400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x1400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            
            "0x0400".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
            "0x0000".to_string(),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_with_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let result = read_hex_data_from_text_file(temp_file.path()).unwrap();
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_parser_with_file_containing_only_comments() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = r#"
# This is a comment
# Another comment
# And another comment
"#;

        write!(temp_file, "{}", content).unwrap();

        let result = read_hex_data_from_text_file(temp_file.path()).unwrap();

        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_read_binary_data_from_file_with_valid_input() {
        let mut temp_file = NamedTempFile::new().unwrap();
        
        // Writing binary data (equivalent to hex values)
        let content: Vec<u8> = vec![
            0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            0x0C, 0x01, 0x00, 0x00, 0x00, 0x0C, 0x02, 0x00,
            0x00, 0x00, 0x0C, 0x03, 0x00, 0x00, 0x00, 0x04,
            0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x14,
            0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x14,
            0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x14,
            0x00, 0x00, 0x00, 0x04
        ];
        
        temp_file.write_all(&content).unwrap();

        let result = read_binary_data_from_file(temp_file.path()).unwrap();

        assert_eq!(result, content);
    }

    #[test]
    fn test_read_binary_data_from_file_with_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let result = read_binary_data_from_file(temp_file.path()).unwrap();
        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn test_read_binary_data_from_file_with_nonexistent_file() {
        let result = read_binary_data_from_file("nonexistent_file.bin");
        assert!(result.is_err()); // Expecting an error since the file doesn't exist
    }
}
