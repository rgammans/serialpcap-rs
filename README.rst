serialpcap-rs
==============

A Rust application for capturing and analyzing serial port data in PCAP format.
Inspired by `SerialPCAP <https://github.com/j123b567/SerialPCAP>`_.

Features
--------
* Capture serial port data in real-time
* Save captures in PCAP format
* Support for common baud rates
* Command-line interface

Installation
------------
To install from source::

    cargo install --git https://github.com/rgammans/serialpcap-rs

Usage
-----
Basic usage::

    serialpcap-rs <PORT> <BAUDRATE> <OUTPUT_FILE>

Example::

    serialpcap-rs /dev/ttyUSB0 115200 capture.pcap

License
-------
This project is licensed under the MIT License - see the LICENSE file for details.

Contributing
-----------
Contributions are welcome! Please feel free to submit a Pull Request.
