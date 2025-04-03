use std::collections::HashMap;
use std::ptr::null_mut;
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

use crate::vlfd::{
    device_handler,
    ProgramHandler,
    helper::*,
    structs::{UsbDevice, UsbHandle}
};
use crate::helper::{smims_cfg, cli_commands};

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
        "discover_fde" => {
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

                let mut buffer = [0u16; 4];
                let mut out_buffer = [0u16; 4];
                let _ = device_handler.io_write_read_data(&mut buffer, &mut out_buffer);

                let t: Vec<_> = out_buffer.iter().rev().flat_map(|&num| (0..16).map(move |i| ((num >> (15 - i)) & 1) as u16)).collect();
                println!("{:?}", t);

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

                let bitfile = "name_display_dc_bit.bit";
                if let Err(e) = program_handler
                    .program(std::path::Path::new(&bitfile))
                    .or_else(|e| {
                        program_handler.close_device()?;
                        Err(e)
                    }) {  return Ok(true); }
                
                let _ = program_handler.close_device();
            }
            return Ok(true);
        }
        "exit" => {
            // cleanup
            
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
}

/**
The main CLI app loop
 */
pub fn run_cli() -> Result<()> {
    let mut prompt = Readline::default().prompt()?;
    let threads: ThreadHandle = Arc::new(Mutex::new(HashMap::new()));

    let mut libusb_context: *mut libusb_ffi::libusb_context = null_mut();
    // libusb_get_context(&mut libusb_context);

    // Initialization tasks:
    let mut app_context = AppContext{
        // libusb_context: libusb_context
        fde_devices: Vec::new(),
        fde_handles: Arc::new(Mutex::new(HashMap::new()))
    };

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
