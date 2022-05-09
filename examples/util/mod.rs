// SPDX-License-Identifier: MIT

use intel_spi::{Mapper, SpiDev, PhysicalAddress, VirtualAddress};

use std::{fs, ptr};

pub struct LinuxMapper;

impl Mapper for LinuxMapper {
    unsafe fn map_aligned(&mut self, address: PhysicalAddress, size: usize) -> Result<VirtualAddress, &'static str> {
        let fd = libc::open(
            b"/dev/mem\0".as_ptr() as *const libc::c_char,
            libc::O_RDWR
        );
        if fd < 0 {
            return Err("failed to open /dev/mem")
        }

        let ptr = libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            address.0 as libc::off_t
        );

        libc::close(fd);

        if ptr == libc::MAP_FAILED {
            return Err("failed to map /dev/mem");
        }

        Ok(VirtualAddress(ptr as usize))
    }

    unsafe fn unmap_aligned(&mut self, address: VirtualAddress, size: usize) -> Result<(), &'static str> {
        if libc::munmap(address.0 as *mut libc::c_void, size) == 0 {
            Ok(())
        } else {
            Err("failed to unmap /dev/mem")
        }
    }

    fn page_size(&self) -> usize {
        //TODO: get dynamically
        4096
    }
}

pub unsafe fn get_spi() -> SpiDev<'static, LinuxMapper> {
    static mut LINUX_MAPPER: LinuxMapper = LinuxMapper;
    let mcfg = fs::read("/sys/firmware/acpi/tables/MCFG").expect("failed to read MCFG");
    SpiDev::new(&mcfg, &mut LINUX_MAPPER).expect("failed to get SPI device")
}
