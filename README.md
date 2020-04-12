# led-name-badge: USB LED name badge control tool

This project is currently under **alpha** stage of development.

The USB name badge configuration tool is not available for Linux Desktop PC.
This tool is a CLI tool to configure the USB name badge on Linux Desktop PC.

A few such tools that can run on Linux Desktop PC are created and available on github,
but my tool uses fonts on the PC to render text using freetype and font-config.

![Rust](https://github.com/cat-in-136/led-name-badge/workflows/Rust/badge.svg)

## How to Build

Dependencies:

* Rust 1.39+
* pkg-config
* libfreetype6
* fontconfig
* hidapi
* libusb

To build, just run `cargo build`.
To run, just run `cargo run -- -h`.

## Reference

* https://lesun-led.en.alibaba.com/productgrouplist-804553412/USB_LED_Name_Badge.html?spm=a2700.icbuShop.88.37.314c615715uv3g
* https://www.youtube.com/results?search_query=led+name+badge
* HappyCodingRobot, [XANES X1 Programmable LED light badge protocoll reverse engineering](https://github.com/HappyCodingRobot/USB_LED_Badge/blob/master/doc/XANESX1ProgrammableLEDlightbadgeprotocollreverseengineering.md)
  * It was created with [HappyCodingRobot/USB_LED_Badge](https://github.com/HappyCodingRobot/USB_LED_Badge) as a reference.

## License

MIT License. See the LICENSE.txt file.
