// SPDX-License-Identifier: MIT

extern crate libc;
extern crate intel_spi;

use intel_spi::Spi;
use std::fs;

mod util;

fn main() {
    let mut spi = unsafe { util::get_spi() };

    eprintln!("SPI HSFSTS_CTL: {:?}", spi.regs.hsfsts_ctl());

    let len = spi.len().unwrap();
    eprintln!("SPI ROM: {} KB", len / 1024);

    let mut data = Vec::with_capacity(len);
    while data.len() < len {
        let mut buf = [0; 65536];
        let read = spi.read(data.len(), &mut buf).unwrap();
        data.extend_from_slice(&buf[..read]);
        eprint!("\rSPI READ: {} KB", data.len() / 1024);
    }

    eprintln!();

    fs::write("read.rom", &data).unwrap();
}
