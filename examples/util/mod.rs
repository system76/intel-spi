use intel_spi::SpiSkl;

use std::{mem, ptr};

pub unsafe fn get_spi() -> &'static mut SpiSkl {
    let spibar = SpiSkl::address();

    let fd = libc::open(
        b"/dev/mem\0".as_ptr() as *const libc::c_char,
        libc::O_RDWR
    );
    if fd < 0 {
        panic!("failed to open /dev/mem");
    }

    let p = libc::mmap(
        ptr::null_mut(),
        mem::size_of::<SpiSkl>(),
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_SHARED,
        fd,
        spibar as libc::off_t
    );
    if p == libc::MAP_FAILED {
        panic!("failed to map /dev/mem");
    }

    libc::close(fd);

    &mut *(p as *mut SpiSkl)
}

pub unsafe fn release_spi(spi: &'static mut SpiSkl) {
    libc::munmap(
        spi as *mut SpiSkl as *mut libc::c_void,
        mem::size_of::<SpiSkl>()
    );
}
