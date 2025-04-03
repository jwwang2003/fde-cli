use tabled::{Table, Tabled, settings::Style};

/// Represents a help entry for a command.
#[derive(Tabled)]
struct CommandHelp {
    /// The command name.
    command: &'static str,
    /// A brief description of the command.
    description: &'static str,
}

/// Prints the list of available commands in a formatted table.
pub fn show_help() {
    let commands = vec![
        CommandHelp {
            command: "help",
            description: "Shows the help message",
        },
        CommandHelp {
            command: "lsusb",
            description: "Lists connected USB devices",
        },
        CommandHelp {
            command: "discover_fde",
            description: "Discover FDE devices",
        },
        CommandHelp {
            command: "configure",
            description: "Setup/load configuration files",
        },
        CommandHelp {
            command: "mount {i}",
            description: "Initialize FDE board {i}"
        },
        // FDE debugging commands
        CommandHelp {
            command: "fde_dump_conf",
            description: "", // Add a description if needed
        },
        CommandHelp {
            command: "lsd",
            description: "Lists connected devices",
        },
        CommandHelp {
            command: "arm",
            description: "Select and arm a libusb device",
        },
        CommandHelp {
            command: "list",
            description: "List currently connected and detected libusb devices",
        },
        CommandHelp {
            command: "reconfigure",
            description: "Go through the constraint and bitstream selection again",
        },
        CommandHelp {
            command: "test",
            description: "Options for testing",
        },
    ];

    // Build the table using a modern style.
    let mut table = Table::new(commands);
    let _table = table.with(Style::modern());

    println!("Available commands:");
    println!("{}", _table);
}