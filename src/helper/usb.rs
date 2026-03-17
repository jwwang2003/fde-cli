use anyhow::Result;
use libusb1_sys as libusb_ffi;
use once_cell::sync::Lazy;
use std::os::raw::c_int;
use std::sync::Mutex;
use vlfd_rs::constants::{DW_PID, DW_VID};

/// Represents a USB device discovered via libusb.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UsbDevice {
    pub bus: i32,
    pub address: i32,
    pub id_vendor: u16,
    pub id_product: u16,
    pub serial_number: u8,
}

pub fn get_usb_devices() -> Result<Vec<UsbDevice>> {
    let mut devices = Vec::new();
    unsafe {
        let context = libusb_get_context();
        let mut device_list: *const *mut libusb_ffi::libusb_device = std::ptr::null_mut();
        let count = libusb_ffi::libusb_get_device_list(context, &mut device_list);
        if count < 0 {
            return Err(anyhow::anyhow!("Error getting device list"));
        }

        for i in 0..count {
            let device = *device_list.offset(i as isize);
            let mut descriptor: libusb_ffi::libusb_device_descriptor = std::mem::zeroed();
            if libusb_ffi::libusb_get_device_descriptor(device, &mut descriptor) < 0 {
                eprintln!("Error getting device descriptor for device {}", i);
                continue;
            }

            devices.push(UsbDevice {
                bus: libusb_ffi::libusb_get_bus_number(device) as i32,
                address: libusb_ffi::libusb_get_device_address(device) as i32,
                id_vendor: descriptor.idVendor,
                id_product: descriptor.idProduct,
                serial_number: descriptor.iSerialNumber,
            });
        }

        libusb_ffi::libusb_free_device_list(device_list, 1);
    }
    Ok(devices)
}

pub fn print_usb_devices() -> Result<()> {
    let devices = get_usb_devices()?;
    println!("Found {} USB devices:", devices.len());
    for device in devices {
        println!(
            "Bus {:03} Device {:03}: ID {:04x}:{:04x} Serial: {:08x}",
            device.bus, device.address, device.id_vendor, device.id_product, device.serial_number
        );
    }
    Ok(())
}

pub fn get_usb_devices_by_vid_pid(vid: u16, pid: u16) -> Result<Vec<UsbDevice>> {
    let mut devices = Vec::new();
    unsafe {
        let context = libusb_get_context();
        let mut device_list: *const *mut libusb_ffi::libusb_device = std::ptr::null_mut();
        let count = libusb_ffi::libusb_get_device_list(context, &mut device_list);
        if count < 0 {
            return Err(anyhow::anyhow!("Error getting device list"));
        }

        for i in 0..count {
            let device = *device_list.offset(i as isize);
            let mut descriptor: libusb_ffi::libusb_device_descriptor = std::mem::zeroed();
            if libusb_ffi::libusb_get_device_descriptor(device, &mut descriptor) < 0 {
                continue;
            }
            if descriptor.idVendor == vid && descriptor.idProduct == pid {
                devices.push(UsbDevice {
                    bus: libusb_ffi::libusb_get_bus_number(device) as i32,
                    address: libusb_ffi::libusb_get_device_address(device) as i32,
                    id_vendor: descriptor.idVendor,
                    id_product: descriptor.idProduct,
                    serial_number: descriptor.iSerialNumber,
                });
            }
        }

        libusb_ffi::libusb_free_device_list(device_list, 1);
    }
    Ok(devices)
}

pub fn ls_usb_smims() -> Result<Vec<UsbDevice>> {
    get_usb_devices_by_vid_pid(DW_VID, DW_PID)
}

struct LibusbContextWrapper(*mut libusb_ffi::libusb_context);

unsafe impl Send for LibusbContextWrapper {}
unsafe impl Sync for LibusbContextWrapper {}

fn libusb_get_context() -> *mut libusb_ffi::libusb_context {
    static GLOBAL_CONTEXT: Lazy<Mutex<LibusbContextWrapper>> = Lazy::new(|| {
        let mut ctx: *mut libusb_ffi::libusb_context = std::ptr::null_mut();
        let ret: c_int = unsafe { libusb_ffi::libusb_init(&mut ctx) };
        if ret < 0 {
            panic!("failed to initialize libusb: error code {}", ret);
        }
        Mutex::new(LibusbContextWrapper(ctx))
    });

    let ctx_wrapper = GLOBAL_CONTEXT.lock().unwrap();
    ctx_wrapper.0
}
