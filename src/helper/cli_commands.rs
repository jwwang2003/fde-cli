/// Prints the list of available commands in a formatted table.
pub fn show_help() {
    let discovery = [
        ("lsusb", "List connected USB devices"),
        ("discover", "Discover FDE devices"),
    ];
    let recipes = [
        ("scan_recipes", "Scan recipes and legacy projects folders"),
        ("ls_recipes", "Show discovered recipies"),
        ("load_recipe <name>", "Load recipie by folder name"),
        ("scan_proj", "Alias of scan_recipes"),
        ("ls_proj", "Alias of ls_recipes"),
        ("load_proj <name>", "Alias of load_recipe"),
    ];
    let fde = [
        ("mount <id>", "Mount an FDE board by index"),
        ("unmount <id>", "Unmount a discovered FDE board"),
        ("fde_list", "List connected FDE boards"),
        ("fde_handles", "List mounted FDE boards"),
        ("fde_dump_conf <id>", "Read and print device configuration"),
        ("program <id>", "Program FPGA using loaded recipie"),
        ("reset <id>", "Erase current FPGA program and reinitialize command mode"),
        ("reset_fdp <id>", "Alias of reset <id>"),
    ];
    let debug = [
        ("attach <id>", "Start interactive debug I/O session"),
        ("detach", "Stop interactive debug I/O session"),
        ("tick [n] [word]", "Run 1 or N clock ticks (default word: 0x0400)"),
        ("tick_file <path>", "Run normal ticks from a word list file"),
        ("tick_freq <hz> [n] [word]", "With n: finite run. Without n: continuous until C"),
        ("send <word> [n]", "Send manual word for 1 or N ticks"),
    ];
    let general = [
        ("help", "Show this help message"),
        ("source <file.tcl>", "Run commands from a Tcl script file"),
        ("configure", "Setup/load configuration files"),
        ("reconfigure", "Reload constraints and bitstream flow"),
        ("quit", "Exit the CLI"),
    ];

    println!("Available commands:");
    println!("\n  Discovery:");
    for (command, description) in discovery {
        println!("  {:<22} {}", command, description);
    }
    println!("\n  Recipies:");
    for (command, description) in recipes {
        println!("  {:<22} {}", command, description);
    }
    println!("\n  FDE Board:");
    for (command, description) in fde {
        println!("  {:<22} {}", command, description);
    }
    println!("\n  Debug:");
    for (command, description) in debug {
        println!("  {:<22} {}", command, description);
    }
    println!("\n  General:");
    for (command, description) in general {
        println!("  {:<22} {}", command, description);
    }
}
