extern crate libc;
extern crate intel_spi;

use intel_spi::{Spi, SpiCnl};
use std::{fs, mem, ptr};

unsafe fn get_spi() -> &'static mut SpiCnl {
    let spibar = 0xfe010000;

    let fd = libc::open(
        b"/dev/mem\0".as_ptr() as *const libc::c_char,
        libc::O_RDWR
    );
    if fd < 0 {
        panic!("failed to open /dev/mem");
    }

    let p = libc::mmap(
        ptr::null_mut(),
        mem::size_of::<SpiCnl>(),
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_SHARED,
        fd,
        spibar
    );
    if p == libc::MAP_FAILED {
        panic!("failed to map /dev/mem");
    }

    libc::close(fd);

    &mut *(p as *mut SpiCnl)
}

unsafe fn release_spi(spi: &'static mut SpiCnl) {
    libc::munmap(
        spi as *mut SpiCnl as *mut libc::c_void,
        mem::size_of::<SpiCnl>()
    );
}

fn main() {
    let spi = unsafe { get_spi() };

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
    let mut i = 0;
    for (chunk, new_chunk) in data.chunks(65536).zip(new.chunks(65536)) {
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

        eprint!("\rSPI WRITE: {} KB", data.len() / 1024);

        if matching {
            // Skip
        } else if erased {
            //TODO: Use erase
            spi.write(i, &new_chunk).unwrap();
        } else {
            spi.write(i, &new_chunk).unwrap();
        }

        i += chunk.len();
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

    unsafe { release_spi(spi); }
}
