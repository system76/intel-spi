extern crate libc;
extern crate intel_spi;

use intel_spi::Spi;
use std::fs;

mod util;

fn main() {
    let spi = unsafe { util::get_spi() };

    eprintln!("SPI HSFSTS_CTL: {:?}", spi.hsfsts_ctl());

    let len = spi.len().unwrap();
    eprintln!("SPI ROM: {} KB", len / 1024);

    let new = fs::read("flash.rom").unwrap();
    if new.len() != len {
        panic!("flash.rom size invalid: {} KB", new.len() / 1024)
    }

    let mut data = Vec::with_capacity(len);
    while data.len() < len {
        let mut buf = [0; 65536];
        let read = spi.read(data.len(), &mut buf).unwrap();
        data.extend_from_slice(&buf[..read]);

        eprint!("\rSPI READ: {} KB", data.len() / 1024);
    }

    eprintln!("");

    //TODO: Get from descriptor somehow
    let erase_byte = 0xFF;
    let erase_size = 4096;
    let mut i = 0;
    for (chunk, new_chunk) in data.chunks(erase_size).zip(new.chunks(erase_size)) {
        // Data matches, meaning sector can be skipped
        let mut matching = true;
        // Data is erased, meaning sector can be erased instead of written
        let mut erased = true;
        for (&byte, &new_byte) in chunk.iter().zip(new_chunk.iter()) {
            if new_byte != byte {
                matching = false;
            }
            if new_byte != erase_byte {
                erased = false;
            }
        }

        if ! matching {
            spi.erase(i).unwrap();
            if ! erased {
                spi.write(i, &new_chunk).unwrap();
            }
        }

        i += chunk.len();

        eprint!("\rSPI WRITE: {} KB", i / 1024);
    }

    eprintln!("");

    data.clear();
    while data.len() < len {
        let mut address = data.len();

        let mut buf = [0; 65536];
        let read = spi.read(address, &mut buf).unwrap();
        data.extend_from_slice(&buf[..read]);

        while address < data.len() {
            if data[address] != new[address] {
                panic!(
                    "verification failed as {:#x}: {:#x} != {:#x}",
                    address,
                    data[address],
                    new[address]
                );
            }
            address += 1;
        }

        eprint!("\rSPI VERIFY: {} KB", data.len() / 1024);
    }

    eprintln!("");

    unsafe { util::release_spi(spi); }
}
