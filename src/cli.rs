use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
type ThreadHandle = Arc<Mutex<HashMap<u64, (thread::JoinHandle<()>, Arc<AtomicBool>)>>>;

use anyhow::Result;

use promkit::preset::readline::Readline;
use owo_colors::OwoColorize;
use tabled::Table;
use tabled::settings::{Style, Alignment, object::Columns};

use libusb1_sys as libusb_ffi;

use crate::ports::{self, table};
use crate::vlfd::{
    device_handler,
    ProgramHandler,
    helper::*,
    structs::{UsbDevice, UsbHandle}
};
use crate::helper::{bitstream, cli_commands, constraints, smims_cfg};
use crate::manager::{self, FileEntry, ScanResult};

pub fn handle_command(command: &str, app_context: &mut AppContext) -> Result<bool> {
    match command.to_lowercase().as_str() {
        "help" => {
            cli_commands::show_help();
            return Ok(true);
        }
        "lsusb" => {
            print_usb_devices()?;
            Ok(true)
        }
        "discover" => {
            println!("Listing {} FDE boards...", "detected".yellow());
            let fde_devices = ls_usb_smims()?;
            for (i, usb_device) in fde_devices.iter().enumerate() {
                println!("{i} | Bus {:03} Device {:03}: ID {:04x}:{:04x} Serial: {:08x}", 
                    usb_device.bus,
                    usb_device.address, 
                    usb_device.id_product, 
                    usb_device.id_vendor, 
                    usb_device.serial_number
                );
            }
            
            app_context.fde_devices = fde_devices;

            println!("{}", "Mount a FDE device by calling `mount i`".yellow());
            
            Ok(true)
        }
        command if command.starts_with("mount ") => {
            // Dump the configuration space for a specific device
            if let Some(id) = command.split_whitespace().nth(1) {
                if !id.is_ascii() {
                    println!("invalid input");
                    return Ok(true);
                }
                let id: u32 = match id.parse() {
                    Ok(id) => id,
                    Err(error) => {
                        println!("{}", error);
                        return Ok(true)
                    },
                };

                // id contains the index of the device descriptor stored in our app state that we want to mount
                // Some checks
                if id as usize > app_context.fde_devices.len() - 1 {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }

                let fde_usb_device = &app_context.fde_devices[id as usize];

                let usb_handle = get_usb_handle(
                    fde_usb_device.bus,
                    fde_usb_device.address,
                    fde_usb_device.id_vendor,
                    fde_usb_device.id_product
                )?;

                let mut handles = app_context.fde_handles.lock().unwrap();
                handles.insert(fde_usb_device.clone(), UsbHandle{ handle:usb_handle, context: libusb_get_context() });
                // app_context.fde_handles.
            }

            Ok(true)
        }
        command if command.starts_with("unmount ") => {
            // Dump the configuration space for a specific device
            if let Some(id) = command.split_whitespace().nth(1) {
                if !id.is_ascii() {
                    println!("invalid input");
                    return Ok(true);
                }
                let id: u32 = match id.parse() {
                    Ok(id) => id,
                    Err(error) => {
                        println!("{}", error);
                        return Ok(true)
                    },
                };

                // id contains the index of the device descriptor stored in our app state that we want to mount
                // Some checks
                if id as usize > app_context.fde_devices.len() - 1 {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }

                let fde_usb_device = &app_context.fde_devices[id as usize];
                let mut handles = app_context.fde_handles.lock().unwrap();
                let fde_handle = &handles[fde_usb_device];

                unsafe {
                    libusb_ffi::libusb_close(fde_handle.handle);
                    // Optionally set the handle to null to prevent double-closing.
                    handles.remove(fde_usb_device);
                }
            }

            Ok(true)
        }
        "configure" => {

            Ok(true)
        }
        
        // ================================================================================================
        // ===================================== PROJECT MANAGER ==========================================
        // ================================================================================================
        "scan_proj" => {
            let scan_result = manager::scan();
            app_context.project_manager = scan_result;
            Ok(true)
        }
        "ls_proj" => {
            println!("Listing {} projects/recipies:", "discovered".yellow());
            let manager_results = &app_context.project_manager;

            if manager_results.projects.len() == 0 {
                println!("{}", "No projects found".red());
            } else {
                println!("{}", "Projects".bold().green());
                println!("{}", serde_json::to_string_pretty(&manager_results.projects).unwrap());
            }
            if manager_results.recipes.len() == 0 {
                println!("{}", "No recipies found".red());
            } else {
                println!("{}", "Recipes".bold().green());
                println!("{}", serde_json::to_string_pretty(&manager_results.recipes).unwrap());
            }
            
            Ok(true)
        }
        command if command.starts_with("load_proj ") => {
            if let Some(pj_id) = command.split_whitespace().nth(1) {
                if !pj_id.is_ascii() {
                    println!("invalid input");
                    return Ok(true);
                }
                let pj_id: String = match pj_id.parse() {
                    Ok(id) => id,
                    Err(error) => {
                        println!("{}", error);
                        return Ok(true)
                    },
                };

                let mut entry = manager::find_file_entry_by_folder(&app_context.project_manager.recipes, &pj_id);
                if entry.is_none() {
                    println!("Looking for project \"{}\"", pj_id);
                    entry = manager::find_file_entry_by_folder(&app_context.project_manager.projects, &pj_id);
                    
                    if entry.is_none() {
                        println!("Project \"{}\" {}", pj_id, "not found!".red());
                        return Ok(true);
                    }

                    println!("{} project {}", "loaded".green(), pj_id.yellow());
                } else {
                    println!("{} recipe {}", "Loaded".green(), pj_id.yellow());
                }

                let entry = entry.unwrap();
                
                // Read & load contraints
                println!("Reading contraints...");
                let mut constraints_loader = constraints::ConstraintsReader::new(entry.cons.to_str().unwrap());
                if let Err(e) = constraints_loader.read() {
                    println!("{} {}", "Something went wrong while reading contraints (.xml) file".red(), e);
                }
                constraints_loader.print_ports();

                // Read & load bitstream file
                println!("Reading bitsream...");
                let mut bitstream_loader = bitstream::ProgramDataReader::new(entry.dc_bit.to_str().unwrap());
                if let Err(e) = bitstream_loader.read() {
                    println!("{} {}", "Something went wrong while reading bitstream (.bit) file".red(), e);
                }
                bitstream_loader.preview_prorgam_data();

                let port_mappings = ports::fde_parse_ports().unwrap();
                let constraints = constraints_loader.get_ports();
                let mut port_vec: Vec<ports::Port> = Vec::new();

                for constraint in constraints.iter() {
                    let new_port = ports::new_port(constraint.clone(), port_mappings.clone());
                    println!("{:?}", new_port);
                    port_vec.push(new_port);
                }

                app_context.io = Some(ports::group_ports(&port_vec, port_mappings));
                app_context.current_project = Some(entry.clone());
            }
            Ok(true)
        }

        // ================================================================================================
        // ===================================== FDE BOARD DEBUG ==========================================
        // ================================================================================================
        command if command.starts_with("fde_dump_conf ") => {
            // Dump the configuration space for a specific device
            if app_context.fde_devices.len() == 0 {
                println!("{}", "No fde_devices found.".yellow());
                return Ok(true);
            }
            if let Some(id) = command.split_whitespace().nth(1) {
                if !id.is_ascii() {
                    println!("invalid input");
                    return Ok(true);
                }
                let id: u32 = match id.parse() {
                    Ok(id) => id,
                    Err(error) => {
                        println!("{}", error);
                        return Ok(true)
                    },
                };

                if id as usize > app_context.fde_devices.len() - 1 {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }

                let fde_usb_device = &app_context.fde_devices[id as usize];
                let handles = app_context.fde_handles.lock().unwrap();

                if handles.len() == 0 {
                    println!("{}", "No fde_handles found.".yellow());
                    return Ok(true);
                }

                let fde_handle = &handles[fde_usb_device];

                let mut device_handler = device_handler::DeviceHandler::new(fde_handle);
                if let Err(e) = device_handler.open() {
                    return Err(anyhow::anyhow!("{}", e));
                }
    
                if let Err(e) = device_handler.init() {
                    return Err(anyhow::anyhow!("{}", e));
                }
    
                let cfg_table = smims_cfg::CfgTable::from_cfg(&device_handler.cfg);
                let mut table = Table::new(cfg_table);
                table.with(Style::modern());
                table.modify(Columns::first(), Alignment::right());
    
                // Use the tabled crate to generate a formatted table string.
                let table_str = table.to_string();
    
                // Print the table.
                println!("{}", table_str);
            }

            Ok(true)
        }
        "fde_handles" => {
            let handles: MutexGuard<_> = app_context.fde_handles.lock().unwrap();
            if handles.is_empty() {
                println!("No fde_handles found.");
                return Ok(true)
            } else {
                println!("Mounted USB handles:");
                for (i, (usb_device, usb_handle)) in handles.iter().enumerate() {
                    println!(
                        "{i} | Device (Bus: {}, Address: {}, VID: {:#04x}, PID: {:#04x}) => Handle: {:?}, Context: {:?}",
                        usb_device.bus, usb_device.address, usb_device.id_vendor, usb_device.id_product, usb_handle.handle, usb_handle.context
                    );
                }
                return Ok(true)
            }
        }

        // ================================================================================================
        // ========================================= FDE BOARD ============================================
        // ================================================================================================
        "fde_list" => {
            println!("Listing {} FDE boards...", "connected".yellow());
            for usb_device in ls_usb_smims()? {
                println!("Bus {:03} Device {:03}: ID {:04x}:{:04x} Serial: {:08x}", 
                    usb_device.bus,
                    usb_device.address, 
                    usb_device.id_product, 
                    usb_device.id_vendor, 
                    usb_device.serial_number
            )   ;
            }
            return Ok(true);
        }

        command if command.starts_with("arm ") => {
            // Arm's a FDE device that is already has its USB connection initiated
            
            if let Some(id) = command.split_whitespace().nth(1) {
                println!("{}", id);
            }

            Ok(true)
        }

        command if command.starts_with("reset ") => {
            if let Some(id) = command.split_whitespace().nth(1) {
                if !id.is_ascii() {
                    println!("invalid input");
                    return Ok(true);
                }
                let id: u32 = match id.parse() {
                    Ok(id) => id,
                    Err(error) => {
                        println!("{}", error);
                        return Ok(true)
                    },
                };
    
                if id as usize > app_context.fde_devices.len() - 1 {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }
    
                let fde_usb_device = &app_context.fde_devices[id as usize];
                let handles = app_context.fde_handles.lock().unwrap();
    
                if handles.len() == 0 {
                    println!("{}", "No fde_handles found.".yellow());
                    return Ok(true);
                }
    
                let fde_handle = &handles[fde_usb_device];
    
                let mut device_handler = device_handler::DeviceHandler::new(fde_handle);
                if let Err(e) = device_handler.open() {
                    return Err(anyhow::anyhow!("{}", e));
                }
    
                if let Err(e) = device_handler.init() {
                    return Err(anyhow::anyhow!("{}", e));
                }

                if let Err(e) = device_handler.engine_reset() {
                    return Err(anyhow::anyhow!("{}", e));
                }
            }

            return Ok(true)
        }
        

        
        "reconfigure" => {
            println!("Reconfiguring constraints and bitstreams...");
            // Implement reconfigure functionality here
            return Ok(true);
        }
        command if command.starts_with("test ") => {
            if let Some(id) = command.split_whitespace().nth(1) {
                if !id.is_ascii() {
                    println!("invalid input");
                    return Ok(true);
                }
                let id: u32 = match id.parse() {
                    Ok(id) => id,
                    Err(error) => {
                        println!("{}", error);
                        return Ok(true)
                    },
                };
    
                if id as usize > app_context.fde_devices.len() - 1 {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }
    
                let fde_usb_device = &app_context.fde_devices[id as usize];
                let handles = app_context.fde_handles.lock().unwrap();
    
                if handles.len() == 0 {
                    println!("{}", "No fde_handles found.".yellow());
                    return Ok(true);
                }
    
                let fde_handle = &handles[fde_usb_device];
                let mut device_handler = device_handler::DeviceHandler::new(fde_handle);
                if let Err(e) = device_handler.open() {
                    // error!("ERROR {}", e); return Ok(true);
                }

                let _ = device_handler.init();

                if let Err(e) = device_handler.io_open() {
                    println!("ERROR {e}");
                }

                // let mut tx_buffer: Vec<u16> = [
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                //     0x0,
                // ].to_vec();
                
                let mut tx_buffer: Vec<u16> = [
                    0x600,
                    0x0,
                    0x0,
                    0x0,

                    0x0,
                    0x0,
                    0x0,
                    0x0,

                    0x200,
                    0x0,
                    0x0,
                    0x0,

                    0x400,
                    0x0,
                    0x0,
                    0x0,

                    0xC01,
                    0x0,
                    0x0,
                    0x0,

                    0xC02,
                    0x0,
                    0x0,
                    0x0,

                    0xC03,
                    0x0,
                    0x0,
                    0x0,

                    0x400,
                    0x0,
                    0x0,
                    0x0,

                    0x400,
                    0x0,
                    0x0,
                    0x0,

                    0x1400,
                    0x0,
                    0x0,
                    0x0,

                    0x1400,
                    0x0,
                    0x0,
                    0x0,

                    0x1400,
                    0x0,
                    0x0,
                    0x0,

                    0x1400,
                    0x0,
                    0x0,
                    0x0,

                    0x1400,
                    0x0,
                    0x0,
                    0x0,

                    0x1400,
                    0x0,
                    0x0,
                    0x0,

                    0x1400,
                    0x0,
                    0x0,
                    0x0,

                    0x400,
                    0x0,
                    0x0,
                    0x0,
                ].to_vec();

                let mut rx_buffer: Vec<u16> = [0u16; 8*7 + 12].to_vec();
                // let mut rx_buffer: Vec<u16> = [0u16; 4 * 4].to_vec();
                let _ = device_handler.io_write_read_data(&mut tx_buffer, &mut rx_buffer);
                for chunk in rx_buffer.chunks_exact_mut(4) {
                    // Convert the current 4 elements into a u64
                    chunk.reverse();
                    let data: u64 = ports::u16_4_to_u64(chunk);
                    if let Some(ref mut current_io) = app_context.io {
                        // Update the value for each port
                        for io in current_io.iter_mut() { io.update(data); }

                        let mut table = Table::new(
                            table::IOPortsTable::from_io(current_io)
                        );
                        table.with(Style::modern());
                        table.modify(Columns::first(), Alignment::right());
                        println!("{}", table.to_string());
                    } else {
                        println!("No IO to update.");
                    }
                }

                let _ = device_handler.io_close();
            }

            return Ok(true);
        }
        command if command.starts_with("program ") => {
            // Dump the configuration space for a specific device
            if app_context.fde_devices.len() == 0 {
                println!("{}", "No fde_devices found.".yellow());
                return Ok(true);
            }
            if let Some(id) = command.split_whitespace().nth(1) {
                let id: u32 = match id.parse() {
                    Ok(id) => id,
                    Err(error) => {
                        println!("{}", error);
                        return Ok(true)
                    },
                };

                if id as usize > app_context.fde_devices.len() - 1 {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }

                let fde_usb_device = &app_context.fde_devices[id as usize];
                let handles = app_context.fde_handles.lock().unwrap();

                if handles.len() == 0 {
                    println!("{}", "No fde_handles found.".yellow());
                    return Ok(true);
                }

                let fde_handle = &handles[fde_usb_device];

                let mut program_handler = ProgramHandler::new(fde_handle);
                if let Err(e) = program_handler.open_device().or_else(|e| {
                    println!("{}", e);
                    program_handler.close_device()?;
                    Err(e)
                }) { return Ok(true) }
                
                if app_context.current_project.is_none() {
                    println!("{}", "No project loaded".red());
                    return Ok(true);
                }

                let current_project = app_context.current_project.clone();
                let bitstream_file = current_project.unwrap().dc_bit;
                if let Err(e) = program_handler
                    .program(std::path::Path::new(&bitstream_file))
                    .or_else(|e| {
                        program_handler.close_device()?;
                        Err(e)
                    }) {  return Ok(true); }
                
                let _ = program_handler.close_device();
            }
            return Ok(true);
        }
        "quit" => {
            // Cleanup
            
            println!("Thanks for using my software...");
            return Ok(false);
        }
        _ => {
            println!("Unknown command: {}, try `help` for commands", command.red());
            return Ok(true);
        }
    }
}

pub fn spawn_thread(threads: &ThreadHandle) {
    // Generate a new unique thread ID using rand crate
    let thread_id = 0;

    // Create an atomic flag that the thread will use to check if it should stop
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Spawn a new thread
    let handle = thread::spawn({
        let stop_flag = Arc::clone(&stop_flag);
        move || {
            loop {
                // Check the stop flag every second
                if stop_flag.load(Ordering::Relaxed) {
                    println!("Thread {}: Stopping...", thread_id);
                    break; // Exit the loop if the stop flag is set
                }
                println!("Thread {}: Printing every second...", thread_id);
                thread::sleep(Duration::from_secs(1));
            }
        }
    });

    // Store the thread handle and the stop flag in the HashMap
    threads.lock().unwrap().insert(thread_id, (handle, stop_flag));
    println!("Spawned thread with ID: {}", thread_id);
}

pub fn kill_thread(threads: &ThreadHandle, thread_id: u64) {
    let mut locked_threads = threads.lock().unwrap();

    if let Some((handle, stop_flag)) = locked_threads.remove(&thread_id) {
        // Set the atomic stop flag to true to signal the thread to stop
        stop_flag.store(true, Ordering::Relaxed);

        // Join the thread to make sure it completes before proceeding
        handle.join().unwrap();
        println!("Killed thread with ID: {}", thread_id);
    } else {
        println!("No thread found with ID: {}", thread_id);
    }
}

pub struct AppContext {
    // Running threads: key is an unique thread id.
    // pub threads: ThreadHandle,

    // List of detected USB devices.
    pub fde_devices: Vec<UsbDevice>,
    // /// Opened USB handles, keyed by (bus, address, vid, pid).
    pub fde_handles: Arc<Mutex<HashMap<UsbDevice, UsbHandle>>>,

    // Project/recipe manager
    pub project_manager: ScanResult,

    pub current_project: Option<FileEntry>,
    pub io: Option<Vec<ports::IOPort>>
}


/// The main CLI app loop
pub fn run_cli() -> Result<()> {
    let mut prompt = Readline::default().prompt()?;
    let threads: ThreadHandle = Arc::new(Mutex::new(HashMap::new()));

    // Initialization tasks:
    let mut app_context = AppContext{
        // libusb_context: libusb_context
        fde_devices: Vec::new(),
        fde_handles: Arc::new(Mutex::new(HashMap::new())),
        project_manager: ScanResult{ projects: Vec::new(), recipes: Vec::new() },
        current_project: None,
        io: None
    };
    // Scan & load projects/recipes
    app_context.project_manager = manager::scan();

    loop {
        // Show the shell prompt
        match prompt.run() {
            Ok(command) => {
                // If the user enters a command, run it
                if !command.trim().is_empty() {
                    if !handle_command(&command, &mut app_context)? {
                        // returning false -> exit
                        break;
                    }
                }
            }
            Err(_) => {
                // If an error occurs with the prompt, exit the shell
                println!("Error reading input, exiting.");
                break;
            }
        }
    }

    Ok(())
}

fn print_bits(rx_buffer: Vec<u16>) {
    let mut current_bits = String::new();
    let mut bit_count = 0;

    for num in rx_buffer {
        // Convert each u16 to a 16-bit binary string
        let binary_str = format!("{:016b}", num);
        
        // Append the binary string to current_bits
        current_bits.push_str(&binary_str);
        bit_count += 16;

        // If we've reached 64 bits, print the current row
        if bit_count == 64 {
            println!("{}", current_bits);
            current_bits.clear();
            bit_count = 0;
        }
    }

    // If there are leftover bits that didn't complete a full 64-bit row, print them
    if bit_count > 0 {
        println!("{}", current_bits);
    }
}