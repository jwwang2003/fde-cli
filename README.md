# fde_cli

A Rust based CLI app that interacts with the SMIMS engine for FDP3PZ.

This CLI offers basic features such as programming bitstream, reading & writing internal SMIMS registers and the FIFO to interact with the FGPA.

> Do not remove `name_display` in the recicpes, it is used in the unit tests.
> The unit tests will not function properly without it.

## Features

- Easy to use CLI interface \(GDB like\)
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


## Credits


## License

Copyright (c) JUN WEI WANG <wjw_03@outlook.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE
