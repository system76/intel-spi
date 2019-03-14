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

    &mut *(p as *mut SpiCnl)
}

fn main() {
    let spi = unsafe { get_spi() };

    let hsfsts_ctl = spi.hsfsts_ctl();
    println!("{:X}: {:?}", hsfsts_ctl.bits(), hsfsts_ctl);

    let len = spi.len().unwrap();
    println!("SPI ROM: {} KB", len / 1024);

    let mut data = Vec::with_capacity(len);
    while data.len() < len {
        let mut buf = [0; 4096];
        let read = spi.read(data.len(), &mut buf).unwrap();
        data.extend_from_slice(&buf);
        print!("\rSPI READ: {} KB", data.len() / 1024);
    }

    println!("");
}
