#![no_std]
#![feature(core_intrinsics)]

pub use self::io::Io;
mod io;

pub use self::mmio::Mmio;
mod mmio;

pub trait Spi {
    fn len(&mut self) -> usize;

    fn read(&mut self, address: usize, buf: &mut [u8]) -> usize;
}

#[repr(packed)]
pub struct SpiCnl {
    bfpreg: Mmio<u32>,
    hsfsts_ctl: Mmio<u32>,
    faddr: Mmio<u32>,
    dlock: Mmio<u32>,
    fdata: [Mmio<u32>; 16],
    fracc: Mmio<u32>,
    freg: [Mmio<u32>; 6],
    _reserved1: [Mmio<u32>; 6],
    fpr: [Mmio<u32>; 5],
    gpr: Mmio<u32>,
    _reserved2: [Mmio<u32>; 5],
    sfracc: Mmio<u32>,
    fdoc: Mmio<u32>,
    fdod: Mmio<u32>,
    _reserved3: Mmio<u32>,
    afc: Mmio<u32>,
    vscc0: Mmio<u32>,
    vscc1: Mmio<u32>,
    ptinx: Mmio<u32>,
    ptdata: Mmio<u32>,
    sbrs: Mmio<u32>,
}

impl Spi for SpiCnl {
    fn len(&mut self) -> usize {
        0
    }

    fn read(&mut self, address: usize, buf: &mut [u8]) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::SpiCnl;

    #[test]
    fn offsets() {
        unsafe {
            let spi: &SpiCnl = &*(0 as *const SpiCnl);

            assert_eq!(&spi.bfpreg as *const _ as usize, 0x00);

            assert_eq!(&spi.freg as *const _ as usize, 0x54);
            assert_eq!(&spi.fpr as *const _ as usize, 0x84);

            assert_eq!(&spi.gpr as *const _ as usize, 0x98);
            assert_eq!(&spi.sfracc as *const _ as usize, 0xb0);

            assert_eq!(&spi.fdod as *const _ as usize, 0xb8);
            assert_eq!(&spi.afc as *const _ as usize, 0xc0);

            assert_eq!(&spi.sbrs as *const _ as usize, 0xd4);
        }
    }
}
