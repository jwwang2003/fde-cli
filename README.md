# fde_cli

A Rust based CLI app that interacts with the SMIMS engine for FDP3PZ.

This CLI offers basic features such as programming bitstream, reading & writing internal SMIMS registers and the FIFO to interact with the FGPA.

## Features

- A modern & well-designed CLI UI
- Easy to use CLI interface (without UI, if the user wishes to use it for automation)
- Multi-threading
- Error handling
- Built-in debuging features


## Project Structure


## File Structure

```
.
├── src
|   ├── main.rs (entry point for cli application or UI)
│   ├── vlfd (reimplementation of SMIMS driver in rust)
│   ├── images
│   │   ├── icons
│   │   │   ├── shrink-button.png
│   │   │   └── umbrella.png
│   │   ├── logo_144.png
│   │   └── Untitled-1.psd
│   └── javascript
│       ├── index.js
│       └── rate.js
├── SMIMS_VLFD (original SMIMS driver in C++)
└── README.md
```

## VLFD reimplementation

Here is exhustive list of the methods that is implemented in the C++ version of the driver.

### Implementation checklist

- [x] `SMIMS_EngineReset`

#### ezIF (Easy Interface)

The EasyInterface (used by Rabbit) is wrapper ontop of the SMIMS FIFO.

- [x] `VLFD_IO_ProrgamFPGA`
- [x] `VLFD_IO_Open` (device_handler/io_open)
- [x] `VLFD_IO_WriteReadData` (device_handler/io_write_read_data)
- [x] `VLFD_IO_Close` (device_handler/io_close)

> It seems that the `ProgramFPGA` function and `IO_Open` are separate. They work independently, because right after programming the FPGA, if I attempt to do any writes to the FIFO, it causes the board to hang.
> 

> After doing some digging in the code, I realized there is a lot more configuration done in the `VLFD_IO_Open` method to set up the device for read and write FIFO.
>
> The reason why the board "hangs" is because of the encryption and licence not being set up properly in the configuration space registers.

> In the CPP driver, there are two identical functions for programming the board:
> - `VLFD_ProgramFPGA`
> - `VLFD_IO_ProgramFPGA`

#### Encrypt API

- [x] `SMIMS_EncryptTableRead` (device_handler/encrypt_table_read)
- [x] `SMIMS_EncryptTableDecode` (device_handler/decoded_encrypt_table)
- [x] `SMIMS_EncryptData` (device_handler/encrypt)
- [x] `SMIMS_DecryptData` (device_handler/decrypt)
- [ ] `SMIMS_EncryptCopy` (Optional)
- [ ] `SMIMS_DecryptCopy` (Optional)
- [ ] `SMIMS_LicenseGen` (Optional)

> The "copy" methods has a separate source and destination because there are pointers, while the regular one uses "in-place" modification. Not sure if it necessary to implmement both in Rust. One of them should be enough for now.

#### Data Trasnfer API

- [x] `SMIMS_FIFO_Write` (device_handler/fifo_write)
- [x] `SMIMS_FIFO_Read` (device_handler/fifo_read)

#### Command API

- [x] `SMIMS_SyncDelay` (device_handler/sync_delay)
- [x] `SMIMS_CommandActive` (device_handler/command_active)
- [x] `SMIMS_CFGSpaceRead` (device_handler/read_cfg)
- [x] `SMIMS_CFGSpaceWrite` (device_handler/write_cfg)
- [x] `SMIMS_FGPAProgrammerActive` (device_handler/activate_fpga_programmer)
- [x] `SMIMS_VeriCommActive` (device_handler/activate_vericomm)
- [x] `SMIMS_VeriInstrumentActive` (device_handler/activate_veri_instrument)
- [x] `SMIMS_VeriLinkActive` (device_handler/activate_verilink)
- [x] `SMIMS_VeriSoCActive` (device_handler/activate_veri_soc)
- [x] `SMIMS_VeriCommProActive` (device_handler/activate_vericomm_pro)
- [x] `SMIMS_VeriSDKActive` (device_handler/activate_veri_sdk)
- [x] `SMIMS_FlashReadActive` (device_handler/activate_flash_read)
- [x] `SMIMS_FlashWriteActive` (device_handler/activate_flash_write)

#### Configuration Space API

Configuration space APIs are implemented in the `cfg` module.

- [x] `SMIMS_GetVeriComm_ClockHighDelay`
- [x] `SMIMS_GetVeriComm_ClockLowDelay`
- [x] `SMIMS_GetVeriComm_ISV`
- [x] `SMIMS_IsVeriComm_ClockCheck_Enable`
- [x] `SMIMS_GetVeriSDK_ChannelSelector`
- [x] `SMIMS_GetModeSelector`
- [x] `SMIMS_GetFlashBeginBlockAddr`
- [x] `SMIMS_GetFlashBeginClusterAddr`
- [x] `SMIMS_GetFlashReadEndBlockAddr`
- [x] `SMIMS_GetFlashReadEndClusterAddr`
- [x] `SMIMS_GetSecurityKey` (Optional)
- [x] `SMIMS_SetVeriComm_ClockHighDelay`
- [x] `SMIMS_SetVeriComm_ClockLowDelay`
- [x] `SMIMS_SetVeriComm_ISV`
- [x] `SMIMS_SetVeriComm_ClockCheck`
- [x] `SMIMS_SetVeriSDK_ChannelSelector`
- [x] `SMIMS_SetModeSelector`
- [x] `SMIMS_SetFlashBeginBlockAddr`
- [x] `SMIMS_SetFlashBeginClusterAddr`
- [x] `SMIMS_SetFlashReadEndBlockAddr`
- [x] `SMIMS_SetFlashReadEndClusterAddr`
- [x] `SMIMS_SetLicenseKey`
- [x] `SMIMS_Version`
- [x] `SMIMS_MajorVersion`
- [x] `SMIMS_SubVersion`
- [x] `SMIMS_SubSubVersion`
- [x] `SMIMS_GetFIFOSize`
- [x] `SMIMS_GetFlashTotalBlock`
- [x] `SMIMS_GetFlashBlockSize`
- [x] `SMIMS_GetFlashClusterSize`
- [x] `SMIMS_VeriCommAbility`
- [x] `SMIMS_VeriInstrumentAbility`
- [x] `SMIMS_VeriLinkAbility`
- [x] `SMIMS_VeriSoCAbility`
- [x] `SMIMS_VeriCommProAbility`
- [x] `SMIMS_VeriSDKAbility`
- [x] `SMIMS_IsFPGAProgram`
- [x] `SMIMS_IsPCBConnect`
- [x] `SMIMS_IsVeriComm_ClockContinue`

#### Serial Functions

## Credits



## License

Copyright (c) JUN WEI WANG <wjw_03@outlook.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE
