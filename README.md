# Mira Screenshare

A high-performance screen-sharing / remote collaboration software written in Rust.

The project is still in its early development phase.

## Build
You need to have ffmpeg installed.

* For macOS, you could use `brew install ffmpeg`.
* For Windows, you need to download ffmpeg from [here](https://github.com/BtbN/FFmpeg-Builds/releases).
Make sure you download a shared library build such as `ffmpeg-n5.1-latest-win64-gpl-5.1.zip`.

Then, simply run `cargo build --release` to build the project. and `cargo run --release` to run it.

## Configure
Configuration file is by default `config.toml`. There are preset configs in `configs/` directory that you could use
as a starting point.

## License

GPLv3

## Attributions
* Some code is adapted from [scrap](https://github.com/quadrupleslap/scrap), which is licensed under the MIT license.
