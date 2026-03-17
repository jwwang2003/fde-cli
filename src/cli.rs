use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use promkit::preset::readline::Readline;
use owo_colors::OwoColorize;
use tabled::builder::Builder;
use tabled::Table;
use tabled::settings::{Style, Alignment, object::Columns};

use crate::ports::{self};
use crate::helper::{bitstream, cli_commands, constraints};
use crate::helper::usb::{ls_usb_smims, print_usb_devices, UsbDevice};
use crate::manager::{self, FileEntry, ScanResult};
use vlfd_rs::{constants, Device, HotplugEventKind, HotplugOptions, HotplugRegistration, IoSettings, Programmer};

pub fn handle_command(command: &str, app_context: &mut AppContext) -> Result<bool> {
    if let Err(e) = refresh_detected_devices(app_context) {
        eprintln!("warning: unable to refresh device list: {}", e);
    }

    if command.starts_with("source ") {
        let raw_path = command["source ".len()..].trim();
        if raw_path.is_empty() {
            println!("usage: source <file.tcl>");
            return Ok(true);
        }

        let script_path = strip_optional_quotes(raw_path);
        return match run_source_script(Path::new(script_path), app_context) {
            Ok(v) => Ok(v),
            Err(e) => {
                println!("source error: {}", e);
                Ok(true)
            }
        };
    }

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
                if id as usize >= app_context.fde_devices.len() {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }

                let fde_usb_device = &app_context.fde_devices[id as usize];

                let mut handles = app_context.fde_handles.lock().unwrap();
                handles.insert(fde_usb_device.clone(), true);
                println!("Mounted device {}", id);
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
                if id as usize >= app_context.fde_devices.len() {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }

                let fde_usb_device = &app_context.fde_devices[id as usize];
                let mut handles = app_context.fde_handles.lock().unwrap();
                handles.remove(fde_usb_device);
                println!("Unmounted device {}", id);
                drop(handles);

                if app_context.debug_target == Some(id) {
                    let _ = close_debug_session(app_context);
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
        "scan_proj" | "scan_recipes" => {
            let scan_result = manager::scan();
            app_context.project_manager = scan_result;
            Ok(true)
        }
        "ls_proj" | "ls_recipes" => {
            println!("Listing {} recipies:", "discovered".yellow());
            let manager_results = &app_context.project_manager;

            if manager_results.recipes.len() == 0 {
                println!("{}", "No recipies found".red());
            } else {
                println!("{}", "Recipies (recipes/)".bold().green());
                println!("{}", serde_json::to_string_pretty(&manager_results.recipes).unwrap());
            }
            if manager_results.projects.len() != 0 {
                println!("{}", "Recipies (legacy projects/)".bold().green());
                println!("{}", serde_json::to_string_pretty(&manager_results.projects).unwrap());
            }
            
            Ok(true)
        }
        command if command.starts_with("load_proj ") || command.starts_with("load_recipe ") => {
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
                    println!("Looking for recipe \"{}\"", pj_id);
                    entry = manager::find_file_entry_by_folder(&app_context.project_manager.projects, &pj_id);
                    
                    if entry.is_none() {
                        println!("Recipe \"{}\" {}", pj_id, "not found!".red());
                        return Ok(true);
                    }

                    println!("{} recipe {} (from projects/)", "loaded".green(), pj_id.yellow());
                } else {
                    println!("{} recipe {}", "loaded".green(), pj_id.yellow());
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
                let connection_types = read_connection_types_from_meta(&entry.meta);
                let constraints = constraints_loader.get_ports();
                let mut port_vec: Vec<ports::Port> = Vec::new();

                for constraint in constraints.iter() {
                    let new_port = ports::new_port(constraint.clone(), port_mappings.clone());
                    port_vec.push(new_port);
                }
                print_loaded_ports(&port_vec);

                app_context.io = Some(ports::group_ports(&port_vec, port_mappings, &connection_types));
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

                if id as usize >= app_context.fde_devices.len() {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }

                let fde_usb_device = &app_context.fde_devices[id as usize];
                let handles = app_context.fde_handles.lock().unwrap();

                if handles.get(fde_usb_device).is_none() {
                    println!("{}", "No fde_handles found.".yellow());
                    return Ok(true);
                }
                drop(handles);

                let mut device = Device::connect().map_err(|e| anyhow::anyhow!("{}", e))?;
                device.read_config().map_err(|e| anyhow::anyhow!("{}", e))?;
                let cfg = device.config();

                let rows = vec![
                    ("SMIMS Version", format!("{:#06x}", cfg.smims_version_raw())),
                    ("Clock High Delay", format!("{:#06x}", cfg.vericomm_clock_high_delay())),
                    ("Clock Low Delay", format!("{:#06x}", cfg.vericomm_clock_low_delay())),
                    ("VeriComm ISV", format!("{:#04x}", cfg.vericomm_isv())),
                    ("Clock Check Enabled", format!("{}", cfg.vericomm_clock_check_enabled())),
                    ("Mode Selector", format!("{:#04x}", cfg.mode_selector())),
                    ("FIFO Size", format!("{:#06x}", cfg.fifo_size())),
                    ("Is Programmed", format!("{}", cfg.is_programmed())),
                    ("PCB Connected", format!("{}", cfg.is_pcb_connected())),
                ];
                let mut table = Table::new(rows);
                table.with(Style::modern());
                table.modify(Columns::first(), Alignment::right());
                println!("{}", table.to_string());
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
                for (i, (usb_device, _)) in handles.iter().enumerate() {
                    println!(
                        "{i} | Device (Bus: {}, Address: {}, VID: {:#04x}, PID: {:#04x})",
                        usb_device.bus, usb_device.address, usb_device.id_vendor, usb_device.id_product
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
        command if command.starts_with("attach ") => {
            if let Some(id_raw) = command.split_whitespace().nth(1) {
                let id = match parse_u32(id_raw) {
                    Ok(id) => id,
                    Err(e) => {
                        println!("{}", e);
                        return Ok(true);
                    }
                };

                if let Err(e) = ensure_mounted(app_context, id) {
                    println!("{}", e);
                    return Ok(true);
                }

                if app_context.debug_device.is_some() {
                    let _ = close_debug_session(app_context);
                }

                let mut device = match Device::connect() {
                    Ok(device) => device,
                    Err(e) => {
                        println!("attach failed: {}", e);
                        return Ok(true);
                    }
                };

                if let Err(e) = device.enter_io_mode(&app_context.debug_settings.clone()) {
                    println!("attach failed: {}", e);
                    return Ok(true);
                }

                app_context.debug_device = Some(device);
                app_context.debug_target = Some(id);
                app_context.debug_needs_flush = true;
                println!("Attached to device {}", id);
            } else {
                println!("usage: attach <id>");
            }
            Ok(true)
        }
        "detach" => {
            if let Err(e) = close_debug_session(app_context) {
                println!("{}", e);
            }
            Ok(true)
        }
        command if command.starts_with("tick_file ") => {
            let raw_path = command["tick_file ".len()..].trim();
            if raw_path.is_empty() {
                println!("usage: tick_file <path>");
                return Ok(true);
            }
            let input_path = strip_optional_quotes(raw_path);

            let words = match read_tick_words_from_file(std::path::Path::new(input_path)) {
                Ok(v) => v,
                Err(e) => {
                    println!("{}", e);
                    return Ok(true);
                }
            };

            if let Err(e) = run_tick_words(app_context, &words) {
                println!("{}", e);
            }
            Ok(true)
        }
        command if command.starts_with("tick") => {
            let args: Vec<&str> = command.split_whitespace().skip(1).collect();
            let (ticks, word) = match args.len() {
                0 => (1usize, app_context.debug_default_word),
                1 => {
                    if is_word_token(args[0]) {
                        let word = match parse_u16(args[0]) {
                            Ok(v) => v,
                            Err(e) => {
                                println!("{}", e);
                                return Ok(true);
                            }
                        };
                        (1usize, word)
                    } else {
                        let ticks = match parse_usize(args[0]) {
                            Ok(v) => v,
                            Err(e) => {
                                println!("{}", e);
                                return Ok(true);
                            }
                        };
                        (ticks, app_context.debug_default_word)
                    }
                }
                2 => {
                    let ticks = match parse_usize(args[0]) {
                        Ok(v) => v,
                        Err(e) => {
                            println!("{}", e);
                            return Ok(true);
                        }
                    };
                    let word = match parse_u16(args[1]) {
                        Ok(v) => v,
                        Err(e) => {
                            println!("{}", e);
                            return Ok(true);
                        }
                    };
                    (ticks, word)
                }
                _ => {
                    println!("usage: tick [n] [word]");
                    return Ok(true);
                }
            };

            if ticks == 0 {
                println!("ticks must be > 0");
                return Ok(true);
            }

            if let Err(e) = run_ticks(app_context, ticks, word) {
                println!("{}", e);
            }
            Ok(true)
        }
        command if command.starts_with("tick_freq ") => {
            let args: Vec<&str> = command.split_whitespace().skip(1).collect();
            if args.is_empty() || args.len() > 3 {
                println!("usage: tick_freq <hz|khz|mhz> [n] [word]");
                return Ok(true);
            }

            let hz = match parse_frequency_hz(args[0]) {
                Ok(v) => v,
                Err(e) => {
                    println!("{}", e);
                    return Ok(true);
                }
            };

            if args.len() == 1 {
                if let Err(e) = run_continuous_frequency(app_context, hz, app_context.debug_default_word) {
                    println!("{}", e);
                }
                return Ok(true);
            }

            let ticks = if args.len() >= 2 {
                match parse_usize(args[1]) {
                    Ok(v) => v,
                    Err(e) => {
                        println!("{}", e);
                        return Ok(true);
                    }
                }
            } else {
                1usize
            };

            let word = if args.len() == 3 {
                match parse_u16(args[2]) {
                    Ok(v) => v,
                    Err(e) => {
                        println!("{}", e);
                        return Ok(true);
                    }
                }
            } else {
                app_context.debug_default_word
            };

            if ticks == 0 {
                println!("ticks must be > 0");
                return Ok(true);
            }

            if let Err(e) = run_ticks_at_frequency(app_context, hz, ticks, word) {
                println!("{}", e);
            }
            Ok(true)
        }
        command if command.starts_with("send ") => {
            let args: Vec<&str> = command.split_whitespace().skip(1).collect();
            if args.is_empty() || args.len() > 2 {
                println!("usage: send <word> [n]");
                return Ok(true);
            }

            let word = match parse_u16(args[0]) {
                Ok(v) => v,
                Err(e) => {
                    println!("{}", e);
                    return Ok(true);
                }
            };
            let ticks = if args.len() == 2 {
                match parse_usize(args[1]) {
                    Ok(v) => v,
                    Err(e) => {
                        println!("{}", e);
                        return Ok(true);
                    }
                }
            } else {
                1usize
            };

            if ticks == 0 {
                println!("ticks must be > 0");
                return Ok(true);
            }

            if let Err(e) = run_ticks(app_context, ticks, word) {
                println!("{}", e);
            }
            Ok(true)
        }

        command if command.starts_with("reset ") || command.starts_with("reset_fdp ") => {
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
    
                if id as usize >= app_context.fde_devices.len() {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }
    
                let fde_usb_device = &app_context.fde_devices[id as usize];
                let handles = app_context.fde_handles.lock().unwrap();
    
                if handles.get(fde_usb_device).is_none() {
                    println!("{}", "No fde_handles found.".yellow());
                    return Ok(true);
                }
                drop(handles);

                if let Err(e) = reset_to_programming_mode(app_context, id) {
                    println!("{}", e);
                    return Ok(true);
                }
                println!(
                    "{}",
                    format!(
                        "Device {} reset complete: FPGA image cleared and command interface reinitialized.",
                        id
                    )
                    .green()
                );
            }

            return Ok(true)
        }
        

        
        "reconfigure" => {
            println!("Reconfiguring constraints and bitstreams...");
            // Implement reconfigure functionality here
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

                if id as usize >= app_context.fde_devices.len() {
                    println!("id {} is out of bounds", id);
                    return Ok(true);
                }

                let fde_usb_device = &app_context.fde_devices[id as usize];
                let handles = app_context.fde_handles.lock().unwrap();

                if handles.get(fde_usb_device).is_none() {
                    println!("{}", "No fde_handles found.".yellow());
                    return Ok(true);
                }
                drop(handles);
                
                if app_context.current_project.is_none() {
                    println!("{}", "No project loaded".red());
                    return Ok(true);
                }

                let current_project = app_context.current_project.clone();
                let bitstream_file = current_project.unwrap().dc_bit;
                let mut programmer = match Programmer::connect() {
                    Ok(programmer) => programmer,
                    Err(e) => {
                        println!("{}", e);
                        return Ok(true);
                    }
                };

                if let Err(e) = programmer.program(std::path::Path::new(&bitstream_file)) {
                    println!("{}", e);
                    let _ = programmer.close();
                    return Ok(true);
                }

                let _ = programmer.close();
            }
            return Ok(true);
        }
        "quit" => {
            // Cleanup
            let _ = close_debug_session(app_context);
            stop_hotplug_notifier(app_context);
            
            println!("Thanks for using my software...");
            return Ok(false);
        }
        _ => {
            println!("Unknown command: {}, try `help` for commands", command.red());
            return Ok(true);
        }
    }
}

fn ensure_mounted(app_context: &AppContext, id: u32) -> std::result::Result<(), String> {
    if id as usize >= app_context.fde_devices.len() {
        return Err(format!("id {} is out of bounds", id));
    }

    let fde_usb_device = &app_context.fde_devices[id as usize];
    let handles = app_context.fde_handles.lock().unwrap();
    if handles.get(fde_usb_device).is_none() {
        return Err("device is not mounted; run `mount <id>` first".to_string());
    }
    Ok(())
}

fn run_ticks_at_frequency(
    app_context: &mut AppContext,
    hz: f64,
    ticks: usize,
    word: u16,
) -> std::result::Result<(), String> {
    if hz <= 0.0 || hz > 50_000_000.0 {
        return Err("frequency must be in (0, 50MHz]".to_string());
    }

    println!(
        "{}",
        format!(
            "Running {} ticks at target {:.6} MHz (Rabbit-style batched scheduling)",
            ticks,
            hz / 1_000_000.0
        )
        .green()
    );
    let verbose = hz <= 500.0 && ticks <= 100;
    if !verbose {
        println!(
            "{}",
            "High-frequency mode: compact output enabled to keep USB loop stable."
                .yellow()
        );
    }

    const TIME_SPLIT_PER_SEC: usize = 200;
    let scheduler_hz = hz.round().max(1.0) as usize;
    let access_usb_times = allocate_access_usb_times(scheduler_hz, TIME_SPLIT_PER_SEC);
    let slot_period = std::time::Duration::from_secs_f64(1.0 / TIME_SPLIT_PER_SEC as f64);

    let mut next_deadline = std::time::Instant::now() + slot_period;
    let mut emitted_ticks = 0usize;
    let mut slot_index = 0usize;
    while emitted_ticks < ticks {
        let now = std::time::Instant::now();
        if now < next_deadline {
            std::thread::sleep(next_deadline - now);
        }

        let mut batch_ticks = access_usb_times[slot_index];
        slot_index = (slot_index + 1) % TIME_SPLIT_PER_SEC;
        if batch_ticks == 0 {
            next_deadline += slot_period;
            continue;
        }
        batch_ticks = batch_ticks.min(ticks - emitted_ticks);

        let frames = match transfer_tick_batch_chunked(app_context, word, batch_ticks) {
            Ok(v) => v,
            Err(e) => {
                if is_timeout_error(&e) {
                    eprintln!("warning: tick timeout, attempting IO recovery...");
                    recover_debug_session(app_context)?;
                    transfer_tick_batch_chunked(app_context, word, batch_ticks)?
                } else {
                    return Err(e);
                }
            }
        };

        for data in frames {
            emitted_ticks += 1;
            if verbose {
                print_tick_frame(emitted_ticks, word, data);

                if let Some(ref mut current_io) = app_context.io {
                    for io in current_io.iter_mut() {
                        io.update(data);
                    }
                    print_io_table(current_io, &mut app_context.text_lcd_state);
                } else {
                    println!("No IO to update.");
                }
            } else if emitted_ticks % 100 == 0 || emitted_ticks == ticks {
                println!(
                    "progress {:>6}/{} | last rx: {:#018x}",
                    emitted_ticks, ticks, data
                );
            }
        }

        // Do not "catch up" with back-to-back transfers if we fall behind.
        // Rabbit's timer model advances in real time, so keep one slot pacing.
        next_deadline = std::time::Instant::now() + slot_period;
    }

    if !verbose {
        println!("{}", "tick_freq completed".green());
    }

    Ok(())
}

fn run_continuous_frequency(
    app_context: &mut AppContext,
    hz: f64,
    word: u16,
) -> std::result::Result<(), String> {
    if hz <= 0.0 || hz > 50_000_000.0 {
        return Err("frequency must be in (0, 50MHz]".to_string());
    }

    let mut device = app_context
        .debug_device
        .take()
        .ok_or_else(|| "no debug session; run `attach <id>` first".to_string())?;

    if app_context.debug_needs_flush {
        flush_debug_io(&mut device)?;
        app_context.debug_needs_flush = false;
    }

    let settings = app_context.debug_settings.clone();
    let running = Arc::new(AtomicBool::new(true));
    let running_worker = Arc::clone(&running);

    println!(
        "{}",
        format!(
            "Continuous tick run at {:.6} MHz started. Press C to stop.",
            hz / 1_000_000.0
        )
        .green()
    );

    let worker = std::thread::spawn(move || {
        run_continuous_frequency_worker(device, settings, hz, word, running_worker)
    });

    let key_wait = wait_for_stop_key(&running);
    running.store(false, Ordering::SeqCst);

    let worker_out = worker
        .join()
        .map_err(|_| "continuous tick worker panicked".to_string())?;

    app_context.debug_device = Some(worker_out.device);
    app_context.debug_needs_flush = worker_out.needs_flush;

    if let Err(e) = key_wait {
        return Err(e);
    }
    if let Some(e) = worker_out.error {
        return Err(e);
    }

    println!(
        "{}",
        format!("Stopped continuous tick run after {} ticks", worker_out.ticks).green()
    );
    Ok(())
}

struct ContinuousWorkerResult {
    device: Device,
    ticks: usize,
    needs_flush: bool,
    error: Option<String>,
}

fn run_continuous_frequency_worker(
    mut device: Device,
    settings: IoSettings,
    hz: f64,
    word: u16,
    running: Arc<AtomicBool>,
) -> ContinuousWorkerResult {
    const TIME_SPLIT_PER_SEC: usize = 200;
    let scheduler_hz = hz.round().max(1.0) as usize;
    let access_usb_times = allocate_access_usb_times(scheduler_hz, TIME_SPLIT_PER_SEC);
    let slot_period = std::time::Duration::from_secs_f64(1.0 / TIME_SPLIT_PER_SEC as f64);

    let mut ticks = 0usize;
    let mut slot_index = 0usize;
    let mut next_deadline = std::time::Instant::now() + slot_period;
    let mut error: Option<String> = None;
    let mut needs_flush = false;

    while running.load(Ordering::Relaxed) {
        let now = std::time::Instant::now();
        if now < next_deadline {
            std::thread::sleep(next_deadline - now);
        }

        let batch_ticks = access_usb_times[slot_index];
        slot_index = (slot_index + 1) % TIME_SPLIT_PER_SEC;
        if batch_ticks == 0 {
            next_deadline = std::time::Instant::now() + slot_period;
            continue;
        }

        let transfer = transfer_tick_batch_for_device(&mut device, word, batch_ticks);
        if let Err(e) = transfer {
            if is_timeout_error(&e) {
                match recover_device_for_worker(&mut device, &settings) {
                    Ok(_) => match transfer_tick_batch_for_device(&mut device, word, batch_ticks) {
                        Ok(_) => {
                            ticks += batch_ticks;
                        }
                        Err(e2) => {
                            error = Some(e2);
                            needs_flush = true;
                            break;
                        }
                    },
                    Err(recover_err) => {
                        error = Some(recover_err);
                        needs_flush = true;
                        break;
                    }
                }
            } else {
                error = Some(e);
                needs_flush = true;
                break;
            }
        } else {
            ticks += batch_ticks;
        }

        next_deadline = std::time::Instant::now() + slot_period;
    }

    ContinuousWorkerResult {
        device,
        ticks,
        needs_flush,
        error,
    }
}

fn transfer_tick_batch_for_device(
    device: &mut Device,
    word: u16,
    batch_ticks: usize,
) -> std::result::Result<(), String> {
    const MAX_BATCH_TICKS_PER_TRANSFER: usize = 64;
    let mut remaining = batch_ticks;
    while remaining > 0 {
        let step = remaining.min(MAX_BATCH_TICKS_PER_TRANSFER);
        let mut tx = vec![0u16; step * 4];
        for frame in tx.chunks_exact_mut(4) {
            frame[0] = word;
        }
        let mut rx = vec![0u16; step * 4];
        device
            .transfer_io(&mut tx, &mut rx)
            .map_err(|e| format!("tick failed: {}", e))?;
        remaining -= step;
    }
    Ok(())
}

fn recover_device_for_worker(device: &mut Device, settings: &IoSettings) -> std::result::Result<(), String> {
    let _ = device.exit_io_mode();
    let mut new_device = Device::connect().map_err(|e| format!("recovery connect failed: {}", e))?;
    new_device
        .enter_io_mode(settings)
        .map_err(|e| format!("recovery enter io mode failed: {}", e))?;
    flush_debug_io(&mut new_device)?;
    *device = new_device;
    Ok(())
}

#[cfg(unix)]
struct TerminalRawGuard {
    fd: i32,
    original: libc::termios,
    active: bool,
}

#[cfg(unix)]
impl TerminalRawGuard {
    fn enter(fd: i32) -> std::result::Result<Self, String> {
        unsafe {
            let mut term = std::mem::zeroed::<libc::termios>();
            if libc::tcgetattr(fd, &mut term) != 0 {
                return Err("failed to read terminal attributes".to_string());
            }
            let original = term;
            term.c_lflag &= !(libc::ICANON | libc::ECHO);
            term.c_cc[libc::VMIN] = 0;
            term.c_cc[libc::VTIME] = 0;
            if libc::tcsetattr(fd, libc::TCSANOW, &term) != 0 {
                return Err("failed to set terminal raw mode".to_string());
            }
            Ok(Self {
                fd,
                original,
                active: true,
            })
        }
    }
}

#[cfg(unix)]
impl Drop for TerminalRawGuard {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                let _ = libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
            }
        }
    }
}

fn wait_for_stop_key(running: &Arc<AtomicBool>) -> std::result::Result<(), String> {
    #[cfg(unix)]
    {
        let _guard = TerminalRawGuard::enter(libc::STDIN_FILENO)?;
        while running.load(Ordering::Relaxed) {
            let mut pfd = libc::pollfd {
                fd: libc::STDIN_FILENO,
                events: libc::POLLIN,
                revents: 0,
            };
            let poll_ret = unsafe { libc::poll(&mut pfd, 1, 100) };
            if poll_ret < 0 {
                return Err("failed polling for stop key".to_string());
            }
            if poll_ret == 0 {
                continue;
            }
            if (pfd.revents & libc::POLLIN) != 0 {
                let mut byte: u8 = 0;
                let read_ret = unsafe {
                    libc::read(
                        libc::STDIN_FILENO,
                        (&mut byte as *mut u8).cast::<libc::c_void>(),
                        1,
                    )
                };
                if read_ret > 0 && (byte == b'c' || byte == b'C') {
                    return Ok(());
                }
            }
        }
        return Ok(());
    }

    #[cfg(not(unix))]
    {
        println!("Type C and press Enter to stop.");
        let mut line = String::new();
        while running.load(Ordering::Relaxed) {
            line.clear();
            let _ = io::stdin().read_line(&mut line);
            if line.trim().eq_ignore_ascii_case("c") {
                return Ok(());
            }
        }
        Ok(())
    }
}

fn transfer_tick_batch_chunked(
    app_context: &mut AppContext,
    word: u16,
    batch_ticks: usize,
) -> std::result::Result<Vec<u64>, String> {
    const MAX_BATCH_TICKS_PER_TRANSFER: usize = 64;

    let mut remaining = batch_ticks;
    let mut out = Vec::with_capacity(batch_ticks);
    while remaining > 0 {
        let step = remaining.min(MAX_BATCH_TICKS_PER_TRANSFER);
        let mut part = transfer_tick_batch(app_context, word, step)?;
        out.append(&mut part);
        remaining -= step;
    }
    Ok(out)
}

fn allocate_access_usb_times(frequency_hz: usize, time_split_per_sec: usize) -> Vec<usize> {
    let mut access_usb_times = vec![0usize; time_split_per_sec];
    let base_times = frequency_hz / time_split_per_sec;
    let remainder_times = frequency_hz % time_split_per_sec;

    if remainder_times != 0 {
        let step = time_split_per_sec / remainder_times;
        for i in 0..remainder_times {
            let idx = i * step;
            if idx < time_split_per_sec {
                access_usb_times[idx] = base_times + 1;
            }
        }
    }

    for slot in access_usb_times.iter_mut() {
        if *slot == 0 {
            *slot = base_times;
        }
    }

    access_usb_times
}

fn transfer_tick_batch(
    app_context: &mut AppContext,
    word: u16,
    batch_ticks: usize,
) -> std::result::Result<Vec<u64>, String> {
    let device = match app_context.debug_device.as_mut() {
        Some(device) => device,
        None => return Err("no debug session; run `attach <id>` first".to_string()),
    };

    if app_context.debug_needs_flush {
        flush_debug_io(device)?;
        app_context.debug_needs_flush = false;
    }

    let mut tx = vec![0u16; batch_ticks * 4];
    for frame in tx.chunks_exact_mut(4) {
        frame[0] = word;
    }
    let mut rx = vec![0u16; batch_ticks * 4];
    device
        .transfer_io(&mut tx, &mut rx)
        .map_err(|e| format!("tick failed: {}", e))?;

    let mut out = Vec::with_capacity(batch_ticks);
    for chunk in rx.chunks_exact(4) {
        let mut frame = chunk.to_vec();
        frame.reverse();
        out.push(ports::u16_4_to_u64(&frame));
    }
    Ok(out)
}

fn is_timeout_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("timed out") || lower.contains("timeout")
}

fn recover_debug_session(app_context: &mut AppContext) -> std::result::Result<(), String> {
    let target = app_context
        .debug_target
        .ok_or_else(|| "no debug target for recovery".to_string())?;
    let settings = app_context.debug_settings.clone();

    if let Some(mut device) = app_context.debug_device.take() {
        let _ = device.exit_io_mode();
    }

    let mut device = Device::connect().map_err(|e| format!("recovery connect failed: {}", e))?;
    device
        .enter_io_mode(&settings)
        .map_err(|e| format!("recovery enter io mode failed: {}", e))?;
    flush_debug_io(&mut device)?;

    app_context.debug_device = Some(device);
    app_context.debug_target = Some(target);
    app_context.debug_needs_flush = false;
    Ok(())
}

fn reset_to_programming_mode(
    app_context: &mut AppContext,
    id: u32,
) -> std::result::Result<(), String> {
    if app_context.debug_target == Some(id) {
        return Err("device is attached for debug; run `detach` first".to_string());
    }

    let mut device = Device::connect().map_err(|e| format!("reset failed: {}", e))?;

    // Equivalent to SMIMS_EngineReset (command 0x02) in VLFDLibUSBDriver.
    device
        .reset_engine()
        .map_err(|e| format!("engine reset failed: {}", e))?;
    let _ = device.close();

    // Re-open once after reset to normalize endpoint state and return to command mode.
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut device = Device::connect().map_err(|e| format!("reopen after reset failed: {}", e))?;
    device
        .command_active()
        .map_err(|e| format!("command mode reactivation failed: {}", e))?;

    device.close().map_err(|e| format!("close failed: {}", e))?;
    Ok(())
}

fn refresh_detected_devices(app_context: &mut AppContext) -> anyhow::Result<()> {
    app_context.fde_devices = ls_usb_smims()?;
    Ok(())
}

fn flush_hotplug_notifications(app_context: &AppContext) {
    let mut queue = app_context.hotplug_messages.lock().unwrap();
    queue.clear();
}

fn start_hotplug_notifications(queue: Arc<Mutex<Vec<String>>>) -> Option<HotplugRegistration> {
    let _ = queue;
    // Hotplug notifications are intentionally disabled for a quieter CLI.
    return None;

    #[allow(unreachable_code)]
    let device = match Device::new() {
        Ok(device) => device,
        Err(e) => {
            eprintln!("warning: failed to initialize hotplug monitor: {}", e);
            return None;
        }
    };

    let options = HotplugOptions {
        vendor_id: Some(constants::DW_VID),
        product_id: Some(constants::DW_PID),
        class_code: None,
        enumerate: true,
    };

    match device.usb().register_hotplug_callback(options, move |event| {
        let msg = match event.kind {
            HotplugEventKind::Arrived => format!(
                "[hotplug] FDE connected: bus {:03} device {:03}",
                event.device.bus_number, event.device.address
            ),
            HotplugEventKind::Left => format!(
                "[hotplug] FDE disconnected: bus {:03} device {:03}",
                event.device.bus_number, event.device.address
            ),
        };

        if let Ok(mut pending) = queue.lock() {
            pending.push(msg);
        }
    }) {
        Ok(registration) => Some(registration),
        Err(e) => {
            eprintln!("warning: failed to register hotplug callback: {}", e);
            None
        }
    }
}

fn start_hotplug_notifier(app_context: &mut AppContext) {
    let _ = app_context;
    // Hotplug notifier output is intentionally disabled.
    return;

    #[allow(unreachable_code)]
    let queue = app_context.hotplug_messages.clone();
    let running = Arc::new(AtomicBool::new(true));
    let running_for_thread = running.clone();

    let handle = thread::spawn(move || {
        while running_for_thread.load(Ordering::Relaxed) {
            let messages = {
                let mut pending = queue.lock().unwrap();
                std::mem::take(&mut *pending)
            };

            if !messages.is_empty() {
                for msg in messages {
                    // Print hotplug notifications immediately and redraw a prompt marker.
                    print!("\r\n{}\r\n❯❯ ", msg);
                }
                let _ = io::stdout().flush();
            }

            thread::sleep(Duration::from_millis(80));
        }
    });

    app_context.hotplug_notifier_running = Some(running);
    app_context.hotplug_notifier_thread = Some(handle);
}

fn stop_hotplug_notifier(app_context: &mut AppContext) {
    if let Some(running) = app_context.hotplug_notifier_running.take() {
        running.store(false, Ordering::SeqCst);
    }
    if let Some(handle) = app_context.hotplug_notifier_thread.take() {
        let _ = handle.join();
    }
}

fn close_debug_session(app_context: &mut AppContext) -> std::result::Result<(), String> {
    if let Some(mut device) = app_context.debug_device.take() {
        if let Err(e) = device.exit_io_mode() {
            app_context.debug_target = None;
            return Err(format!("detach failed: {}", e));
        }
        app_context.debug_target = None;
        println!("Detached");
        return Ok(());
    }
    println!("No active debug session");
    Ok(())
}

fn run_ticks(app_context: &mut AppContext, ticks: usize, word: u16) -> std::result::Result<(), String> {
    let device = match app_context.debug_device.as_mut() {
        Some(device) => device,
        None => return Err("no debug session; run `attach <id>` first".to_string()),
    };

    if app_context.debug_needs_flush {
        flush_debug_io(device)?;
        app_context.debug_needs_flush = false;
    }

    let mut tx_buffer = vec![0u16; ticks * 4];
    let mut rx_buffer = vec![0u16; ticks * 4];

    for frame in tx_buffer.chunks_exact_mut(4) {
        frame[0] = word;
    }

    device
        .transfer_io(tx_buffer.as_mut_slice(), rx_buffer.as_mut_slice())
        .map_err(|e| format!("tick failed: {}", e))?;

    for (i, chunk) in rx_buffer.chunks_exact(4).enumerate() {
        let mut frame = chunk.to_vec();
        frame.reverse();
        let data = ports::u16_4_to_u64(frame.as_mut_slice());
        print_tick_frame(i + 1, word, data);

        if let Some(ref mut current_io) = app_context.io {
            for io in current_io.iter_mut() {
                io.update(data);
            }
            print_io_table(current_io, &mut app_context.text_lcd_state);
        } else {
            println!("No IO to update.");
        }
    }

    Ok(())
}

fn read_tick_words_from_file(path: &std::path::Path) -> std::result::Result<Vec<u16>, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("failed to open input file `{}`: {}", path.display(), e))?;
    let reader = std::io::BufReader::new(file);

    let mut words = Vec::new();
    for (line_idx, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| {
            format!(
                "failed reading input file `{}` at line {}: {}",
                path.display(),
                line_idx + 1,
                e
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let token = trimmed
            .split('#')
            .next()
            .map(|s| s.trim())
            .unwrap_or_default();
        if token.is_empty() {
            continue;
        }

        let word = parse_u16(token)
            .map_err(|e| format!("{} at line {}", e, line_idx + 1))?;
        words.push(word);
    }

    if words.is_empty() {
        return Err(format!("no tick data found in `{}`", path.display()));
    }
    if words.len() % 4 != 0 {
        return Err(format!(
            "input word count must be a multiple of 4, got {} in `{}`",
            words.len(),
            path.display()
        ));
    }

    Ok(words)
}

fn run_tick_words(app_context: &mut AppContext, words: &[u16]) -> std::result::Result<(), String> {
    let device = match app_context.debug_device.as_mut() {
        Some(device) => device,
        None => return Err("no debug session; run `attach <id>` first".to_string()),
    };

    if app_context.debug_needs_flush {
        flush_debug_io(device)?;
        app_context.debug_needs_flush = false;
    }

    let mut tx = words.to_vec();
    let mut rx = vec![0u16; words.len()];
    device
        .transfer_io(tx.as_mut_slice(), rx.as_mut_slice())
        .map_err(|e| format!("tick_file transfer failed: {}", e))?;

    for (i, chunk) in rx.chunks_exact(4).enumerate() {
        let mut frame = chunk.to_vec();
        frame.reverse();
        let data = ports::u16_4_to_u64(frame.as_mut_slice());
        println!(
            "{} {}",
            format!("tick {:>4}", i + 1).bold().cyan(),
            format!("rx: {:#018x}", data).green()
        );

        if let Some(ref mut current_io) = app_context.io {
            for io in current_io.iter_mut() {
                io.update(data);
            }
            print_io_table(current_io, &mut app_context.text_lcd_state);
        }
    }

    Ok(())
}

fn flush_debug_io(device: &mut Device) -> std::result::Result<(), String> {
    // Rabbit's AsyncVLFDReadWrite::checkConnection() sends one 4-word zero frame
    // right after opening I/O mode to stabilize FIFO state.
    let mut tx = [0u16; 4];
    let mut rx = [0u16; 4];
    device
        .transfer_io(&mut tx, &mut rx)
        .map_err(|e| format!("io flush failed: {}", e))?;
    Ok(())
}

fn format_binary_u64_grouped(data: u64) -> String {
    let bits = format!("{:064b}", data);
    let mut grouped = String::with_capacity(71);
    for (i, ch) in bits.chars().enumerate() {
        if i > 0 && i % 8 == 0 {
            grouped.push(' ');
        }
        grouped.push(ch);
    }
    grouped
}

fn print_tick_frame(index: usize, tx_word: u16, rx_data: u64) {
    println!("{}", format!("tick {:>4}", index).bold().cyan());
    println!(
        "{} {}",
        "  tx     :".yellow().bold(),
        format!("{:#06x}", tx_word).yellow()
    );
    println!(
        "{} {}",
        "  rx     :".green().bold(),
        format!("{:#018x}", rx_data).green()
    );
    println!(
        "{} {}",
        "  rx_bin :".magenta().bold(),
        format_binary_u64_grouped(rx_data).magenta()
    );
}

fn print_loaded_ports(ports: &[ports::Port]) {
    println!(
        "{} {}",
        "Loaded recipe ports".green().bold(),
        format!("({})", ports.len()).dimmed()
    );

    let mut builder = Builder::default();
    builder.push_record(vec![
        "name".to_string(),
        "pin".to_string(),
        "pin_index".to_string(),
        "default".to_string(),
    ]);

    for p in ports {
        let pin_index = p.pin_index();
        let pin_index_str = if pin_index < 0 {
            "N/A".to_string()
        } else {
            pin_index.to_string()
        };
        let default_value = if p.value() { "1" } else { "0" };

        builder.push_record(vec![
            p.name().to_string(),
            p.pin_name().to_string(),
            pin_index_str,
            default_value.to_string(),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::modern());
    println!("{}", table);
}

fn print_io_table(current_io: &Vec<ports::IOPort>, text_lcd_state: &mut TextLcdState) {
    let visible_ports: Vec<&ports::IOPort> = current_io
        .iter()
        .filter(|port| !matches!(port.io_type, ports::IOType::DC))
        .collect();

    if visible_ports.is_empty() {
        println!("No non-DC ports to display.");
        return;
    }

    let mut builder = Builder::default();
    let mut header = vec!["field".to_string()];
    header.extend(visible_ports.iter().map(|p| p.io_name.clone()));
    builder.push_record(header);

    let mut row_io_type = vec!["io_type".to_string()];
    row_io_type.extend(visible_ports.iter().map(|p| format!("{}", p.io_type)));
    builder.push_record(row_io_type);

    let mut row_port_name = vec!["port_name".to_string()];
    row_port_name.extend(visible_ports.iter().map(|p| p.io_name.clone()));
    builder.push_record(row_port_name);

    let io_values: HashMap<String, u64> = visible_ports
        .iter()
        .map(|p| (p.io_name.clone(), p.data))
        .collect();

    let mut row_data = vec!["data".to_string()];
    row_data.extend(visible_ports.iter().map(|p| format_port_data(p, &io_values, text_lcd_state)));
    builder.push_record(row_data);

    let mut table = builder.build();
    table.with(Style::modern());
    table.modify(Columns::first(), Alignment::right());
    println!("{}", table.to_string());
}

fn format_port_data(
    port: &ports::IOPort,
    io_values: &HashMap<String, u64>,
    text_lcd_state: &mut TextLcdState
) -> String {
    match port.parse_type {
        ports::ParseDataType::Number => port.data.to_string(),
        ports::ParseDataType::Hex => format!("0x{:x}", port.data),
        ports::ParseDataType::Binary => format!("0b{:b}", port.data),
        ports::ParseDataType::Ascii => {
            if port.data <= 0x7f {
                let byte = port.data as u8;
                if byte.is_ascii_graphic() || byte == b' ' {
                    return format!("'{}'", byte as char);
                }
            }
            format!("0x{:x}", port.data)
        }
        ports::ParseDataType::TextLcd => {
            let en = *io_values.get("lcd_en").unwrap_or(&1);
            let rst = *io_values.get("lcd_rst").unwrap_or(&0);
            let state = text_lcd_state;

            if rst == 1 {
                state.cursor = 0;
                state.buffer.fill(0);
                return "RST".to_string();
            }
            if en == 0 {
                let char_code = (port.data & 0xff) as u8;
                let pos = state.cursor;
                state.buffer[pos] = char_code;
                state.cursor = (state.cursor + 1) % 32;

                let rendered = char_code.saturating_add(0x20);
                if rendered.is_ascii_graphic() || rendered == b' ' {
                    return format!("'{}' @{}", rendered as char, pos);
                }
                return format!("0x{:02x} @{}", char_code, pos);
            }

            "IDLE".to_string()
        }
    }
}

#[derive(Debug, Clone)]
struct TextLcdState {
    cursor: usize,
    buffer: [u8; 32],
}

impl Default for TextLcdState {
    fn default() -> Self {
        Self {
            cursor: 0,
            buffer: [0u8; 32],
        }
    }
}

fn parse_u32(raw: &str) -> std::result::Result<u32, String> {
    raw.parse::<u32>()
        .map_err(|e| format!("invalid number `{}`: {}", raw, e))
}

fn parse_usize(raw: &str) -> std::result::Result<usize, String> {
    raw.parse::<usize>()
        .map_err(|e| format!("invalid number `{}`: {}", raw, e))
}

fn parse_u16(raw: &str) -> std::result::Result<u16, String> {
    if let Some(hex) = raw.strip_prefix("0x") {
        return u16::from_str_radix(hex, 16)
            .map_err(|e| format!("invalid hex word `{}`: {}", raw, e));
    }
    raw.parse::<u16>()
        .map_err(|e| format!("invalid word `{}`: {}", raw, e))
}

fn parse_frequency_hz(raw: &str) -> std::result::Result<f64, String> {
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered.is_empty() {
        return Err("frequency cannot be empty".to_string());
    }

    let (num, mul) = if let Some(v) = lowered.strip_suffix("mhz") {
        (v.trim(), 1_000_000.0)
    } else if let Some(v) = lowered.strip_suffix("khz") {
        (v.trim(), 1_000.0)
    } else if let Some(v) = lowered.strip_suffix("hz") {
        (v.trim(), 1.0)
    } else {
        (lowered.as_str(), 1.0)
    };

    let base = num
        .parse::<f64>()
        .map_err(|e| format!("invalid frequency `{}`: {}", raw, e))?;
    let hz = base * mul;

    if !(hz.is_finite()) || hz <= 0.0 {
        return Err("frequency must be > 0".to_string());
    }
    if hz > 50_000_000.0 {
        return Err("frequency must be <= 50 MHz".to_string());
    }

    Ok(hz)
}

fn is_word_token(raw: &str) -> bool {
    raw.starts_with("0x")
}

fn read_connection_types_from_meta(meta_path: &std::path::Path) -> HashMap<String, String> {
    let meta_json = match std::fs::read_to_string(meta_path) {
        Ok(value) => value,
        Err(_) => return HashMap::new(),
    };

    let meta: serde_json::Value = match serde_json::from_str(&meta_json) {
        Ok(value) => value,
        Err(_) => return HashMap::new(),
    };

    let mut result = HashMap::new();
    let Some(obj) = meta.get("connection_types").and_then(|v| v.as_object()) else {
        return result;
    };

    for (k, v) in obj {
        if let Some(vs) = v.as_str() {
            result.insert(k.to_string(), vs.to_string());
        }
    }

    result
}

pub struct AppContext {
    // Running threads: key is an unique thread id.
    // pub threads: ThreadHandle,

    // List of detected USB devices.
    pub fde_devices: Vec<UsbDevice>,
    // /// Opened USB handles, keyed by (bus, address, vid, pid).
    pub fde_handles: Arc<Mutex<HashMap<UsbDevice, bool>>>,

    // Project/recipe manager
    pub project_manager: ScanResult,

    pub current_project: Option<FileEntry>,
    pub io: Option<Vec<ports::IOPort>>,
    pub debug_device: Option<Device>,
    pub debug_target: Option<u32>,
    pub debug_default_word: u16,
    pub debug_settings: IoSettings,
    pub debug_needs_flush: bool,
    pub hotplug_registration: Option<HotplugRegistration>,
    pub hotplug_messages: Arc<Mutex<Vec<String>>>,
    pub hotplug_notifier_running: Option<Arc<AtomicBool>>,
    pub hotplug_notifier_thread: Option<thread::JoinHandle<()>>,
    pub active_source_scripts: Vec<PathBuf>,
    text_lcd_state: TextLcdState,
}


/// The main CLI app loop
pub fn run_cli() -> Result<()> {
    let mut prompt = Readline::default().enable_history().prompt()?;
    let hotplug_messages = Arc::new(Mutex::new(Vec::<String>::new()));

    // Initialization tasks:
    let mut app_context = AppContext{
        // libusb_context: libusb_context
        fde_devices: Vec::new(),
        fde_handles: Arc::new(Mutex::new(HashMap::new())),
        project_manager: ScanResult{ projects: Vec::new(), recipes: Vec::new() },
        current_project: None,
        io: None,
        debug_device: None,
        debug_target: None,
        debug_default_word: 0x0400,
        debug_settings: IoSettings::default(),
        debug_needs_flush: false,
        hotplug_registration: None,
        hotplug_messages: hotplug_messages.clone(),
        hotplug_notifier_running: None,
        hotplug_notifier_thread: None,
        active_source_scripts: Vec::new(),
        text_lcd_state: TextLcdState::default(),
    };
    // Scan & load projects/recipes
    app_context.project_manager = manager::scan();
    let _ = refresh_detected_devices(&mut app_context);
    app_context.hotplug_registration = start_hotplug_notifications(hotplug_messages);
    start_hotplug_notifier(&mut app_context);
    flush_hotplug_notifications(&app_context);

    loop {
        flush_hotplug_notifications(&app_context);
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
                flush_hotplug_notifications(&app_context);
            }
            Err(_) => {
                // If an error occurs with the prompt, exit the shell
                stop_hotplug_notifier(&mut app_context);
                println!("Error reading input, exiting.");
                break;
            }
        }
    }

    Ok(())
}

fn strip_optional_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let quoted = (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'');
        if quoted {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn run_source_script(script_path: &Path, app_context: &mut AppContext) -> Result<bool> {
    let canonical = std::fs::canonicalize(script_path).map_err(|e| {
        anyhow::anyhow!("failed to open script `{}`: {}", script_path.display(), e)
    })?;

    if app_context.active_source_scripts.contains(&canonical) {
        println!(
            "source recursion detected for `{}`; aborting",
            canonical.display()
        );
        return Ok(true);
    }

    let file = std::fs::File::open(&canonical)
        .map_err(|e| anyhow::anyhow!("failed to open script `{}`: {}", canonical.display(), e))?;
    let reader = std::io::BufReader::new(file);
    app_context.active_source_scripts.push(canonical.clone());

    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| {
            anyhow::anyhow!(
                "failed to read script `{}` at line {}: {}",
                canonical.display(),
                idx + 1,
                e
            )
        })?;

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        println!(
            "{} {}",
            format!("[source:{}]", idx + 1).blue().bold(),
            trimmed.bright_white()
        );
        if !handle_command(trimmed, app_context)? {
            app_context.active_source_scripts.pop();
            return Ok(false);
        }
    }

    app_context.active_source_scripts.pop();
    Ok(true)
}
