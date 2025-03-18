// SPDX-License-Identifier: MIT

#![no_std]

#[macro_use]
extern crate bitflags;

use core::{cmp, mem, slice};

pub use self::io::Io;
mod io;

pub use self::mapper::{PhysicalAddress, VirtualAddress, Mapper};
mod mapper;

pub use self::mmio::Mmio;
mod mmio;

pub static PCI_IDS: &[(u16, u16)] = &[
    (0x8086, 0x02A4), // Comet Lake
    (0x8086, 0x06A4), // Comet Lake-H
    (0x8086, 0x43A4), // Tiger Lake-H
    (0x8086, 0x51A4), // Alder Lake-P
    (0x8086, 0x7723), // Arrow Lake-HU
    (0x8086, 0x7A24), // Alder Lake-S
    (0x8086, 0x7E23), // Meteor Lake-HU
    (0x8086, 0x9DA4), // Cannon Lake
    (0x8086, 0xA0A4), // Tiger Lake
    (0x8086, 0xA324), // Cannon Lake-H
    (0x8086, 0xA324), // Cannon Lake-H
];

#[derive(Debug)]
pub enum SpiError {
    /// Access Error Log (H_AEL) is set
    Access,
    /// Flash Cycle Error (FCERR) is set
    Cycle,
    /// Register contains unexpected data
    Register,
}

#[allow(clippy::len_without_is_empty)]
pub trait Spi {
    fn len(&mut self) -> Result<usize, SpiError>;

    fn read(&mut self, address: usize, buf: &mut [u8]) -> Result<usize, SpiError>;

    fn erase(&mut self, address: usize) -> Result<(), SpiError>;

    fn write(&mut self, address: usize, buf: &[u8]) -> Result<usize, SpiError>;
}

pub struct SpiDev<'m, M: Mapper> {
    mapper: &'m mut M,
    pub regs: &'m mut SpiRegs,
}

impl<'m, M: Mapper> SpiDev<'m, M> {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new(mcfg: &[u8], mapper: &'m mut M) -> Result<Self, &'static str> {
        let pcie_base =
            (mcfg[0x2c] as usize) |
            (mcfg[0x2d] as usize) << 8 |
            (mcfg[0x2e] as usize) << 16 |
            (mcfg[0x2f] as usize) << 24 |
            (mcfg[0x30] as usize) << 32 |
            (mcfg[0x31] as usize) << 40 |
            (mcfg[0x32] as usize) << 48 |
            (mcfg[0x33] as usize) << 56;

        let mut phys_opt = None;
        {
            let (pcie_bus, pcie_dev, pcie_func) = (0x00, 0x1F, 0x05);
            let pcie_size = 4096;

            let pcie_phys = PhysicalAddress(
                pcie_base |
                (pcie_bus << 20) |
                (pcie_dev << 15) |
                (pcie_func << 12)
            );
            let pcie_virt = mapper.map(pcie_phys, pcie_size)?;
            {
                let pcie_space = slice::from_raw_parts_mut(pcie_virt.0 as *mut u8, pcie_size);

                let vendor_id =
                    (pcie_space[0x00] as u16) |
                    (pcie_space[0x01] as u16) << 8;
                let product_id =
                    (pcie_space[0x02] as u16) |
                    (pcie_space[0x03] as u16) << 8;
                for known_id in PCI_IDS.iter() {
                    if known_id.0 == vendor_id && known_id.1 == product_id {
                        let bar0 =
                            (pcie_space[0x10] as u32) |
                            (pcie_space[0x11] as u32) << 8 |
                            (pcie_space[0x12] as u32) << 16 |
                            (pcie_space[0x13] as u32) << 24;
                        phys_opt = Some(PhysicalAddress(bar0 as usize));
                        break;
                    }
                }
            }
            mapper.unmap(pcie_virt, pcie_size)?;
        }

        let phys = match phys_opt {
            Some(some) => some,
            None => return Err("no supported SPI device found"),
        };
        let virt = mapper.map(phys, mem::size_of::<SpiRegs>())?;
        let regs = &mut *(virt.0 as *mut SpiRegs);

        Ok(Self {
            mapper,
            regs,
        })
    }
}

impl<'m, M: Mapper> Spi for SpiDev<'m, M> {
    fn len(&mut self) -> Result<usize, SpiError> {
        self.regs.len()
    }

    fn read(&mut self, address: usize, buf: &mut [u8]) -> Result<usize, SpiError> {
        self.regs.read(address, buf)
    }

    fn erase(&mut self, address: usize) -> Result<(), SpiError> {
        self.regs.erase(address)
    }

    fn write(&mut self, address: usize, buf: &[u8]) -> Result<usize, SpiError> {
        self.regs.write(address, buf)
    }
}

impl<'m, M: Mapper> Drop for SpiDev<'m, M> {
    fn drop(&mut self) {
        let virt = VirtualAddress(self.regs as *mut SpiRegs as usize);
        let _ = unsafe { self.mapper.unmap(virt, mem::size_of::<SpiRegs>()) };
    }
}

bitflags! {
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct HsfStsCtl: u32 {
        /// Flash Cycle Done
        const FDONE = 1 << 0;
        /// Flash Cycle Error
        const FCERR = 1 << 1;
        /// Access Error Log
        const H_AEL = 1 << 2;

        // Reserved 3:4

        /// SPI Cycle In Progress
        const H_SCIP = 1 << 5;

        // Reserved 6:10

        /// Write Status Disable
        const WRSDIS = 1 << 11;
        /// PRR3 PRR4 Lock-Down
        const PRR34_LOCKDN = 1 << 12;
        /// Flash Descriptor Override Pin-Strap Status
        const FDOPSS = 1 << 13;
        /// Flash Descriptor Valid
        const FDV = 1 << 14;
        /// Flash Configuration Lock-Down
        const FLOCKDN = 1 << 15;
        //// Flash Cycle Go
        const FGO = 1 << 16;

        /// Flash Cycle
        const FCYCLE = 0b1111 << 17;
        const FCYCLE_0 = 1 << 17;
        const FCYCLE_1 = 1 << 18;
        const FCYCLE_2 = 1 << 19;
        const FCYCLE_3 = 1 << 20;

        /// Write Enable Type
        const WET = 1 << 21;

        // Reserved 22:23

        /// Flash Data Byte Count - Programmed with count minus one
        const FDBC = 0b111111 << 24;
        const FDBC_0 = 1 << 24;
        const FDBC_1 = 1 << 25;
        const FDBC_2 = 1 << 26;
        const FDBC_3 = 1 << 27;
        const FDBC_4 = 1 << 28;
        const FDBC_5 = 1 << 29;

        // Reserved 30

        /// Flash SPI SMI# Enable
        const FSMIE = 1 << 31;
    }
}

#[allow(dead_code)]
impl HsfStsCtl {
    fn sanitize(&mut self) {
        // FDONE, FCERR, and H_AEL are cleared if one is written, so they are left untouched

        // H_SCIP, FDOPSS, and FDV are RO; WRSDIS, PRR34_LOCKDN, and FLOCKDN are probably locked,
        // so they are left untouched

        // Clear FGO
        self.remove(Self::FGO);

        // Clear FCYCLE
        self.remove(Self::FCYCLE);

        // Clear WET
        self.remove(Self::WET);

        // Clear FDBC
        self.remove(Self::FDBC);

        // Clear FSMIE
        self.remove(Self::FSMIE);
    }

    fn cycle(&self) -> HsfStsCtlCycle {
        unsafe { mem::transmute((*self & Self::FCYCLE).bits()) }
    }

    fn set_cycle(&mut self, value: HsfStsCtlCycle) {
        *self = (*self & !Self::FCYCLE) | (
            Self::from_bits_truncate(value as u32)
        );
    }

    fn count(&self) -> u8 {
        (((*self & Self::FDBC).bits() >> 24) + 1) as u8
    }

    fn set_count(&mut self, value: u8) {
        *self = (*self & !Self::FDBC) | (
            Self::from_bits_truncate(
                (cmp::max(value, 64).saturating_sub(1) as u32) << 24
            )
        );
    }
}

#[repr(u32)]
pub enum HsfStsCtlCycle {
    /// Read number of bytes in FDBC plus one
    Read = 0x0 << 17,
    /// Reserved - treated as 0x0 read
    Rsvd = 0x1 << 17,
    /// Write number of bytes in FDBC plus one
    Write = 0x2 << 17,
    /// Erase 4096 bytes
    BlockErase = 0x3 << 17,
    /// Erase 65536 bytes
    SectorErase = 0x4 << 17,
    /// Read SFDP
    ReadSfdp = 0x5 << 17,
    /// Read JEDEC ID
    ReadJedec = 0x6 << 17,
    /// Write status
    WriteStatus = 0x7 << 17,
    /// Read status
    ReadStatus = 0x8 << 17,
    /// RPMC Op1
    RpmcOp1 = 0x9 << 17,
    /// Rpmc Op2
    RpmcOp2 = 0xA << 17,
}

#[repr(u32)]
pub enum FdoSection {
    Map = 0b000 << 12,
    Component = 0b001 << 12,
    Region = 0b010 << 12,
    Master = 0b011 << 12
}

#[allow(dead_code)]
#[repr(C)]
pub struct SpiRegs {
    /// BIOS Flash Primary Region
    bfpreg: Mmio<u32>,
    /// Hardware Sequencing Flash Status and Control
    hsfsts_ctl: Mmio<u32>,
    /// Flash Address
    faddr: Mmio<u32>,
    /// Discrete Lock Bits
    dlock: Mmio<u32>,
    /// Flash Data
    fdata: [Mmio<u32>; 16],
    /// Flash Region Access Permissions
    fracc: Mmio<u32>,
    /// Flash Regions
    freg: [Mmio<u32>; 6],
    _reserved1: [Mmio<u32>; 6],
    /// Flash Protected Ranges
    fpr: [Mmio<u32>; 5],
    /// Global Protected Range
    gpr: Mmio<u32>,
    _reserved2: [Mmio<u32>; 5],
    /// Secondary Flash Region Access Permissions
    sfracc: Mmio<u32>,
    /// Flash Descriptor Observability Control
    fdoc: Mmio<u32>,
    /// Flash Descriptor Observability Data
    fdod: Mmio<u32>,
    _reserved3: Mmio<u32>,
    // Additional Flash Control
    afc: Mmio<u32>,
    /// Vendor Specific Capabilities for Component 0
    vscc0: Mmio<u32>,
    /// Vendor Specific Capabilities for Component 1
    vscc1: Mmio<u32>,
    /// Parameter Table Index
    ptinx: Mmio<u32>,
    /// Parameter Table Data
    ptdata: Mmio<u32>,
    /// SPI Bus Requester Status
    sbrs: Mmio<u32>,
}

impl SpiRegs {
    pub fn hsfsts_ctl(&self) -> HsfStsCtl {
        HsfStsCtl::from_bits_truncate(self.hsfsts_ctl.read())
    }

    pub fn set_hsfsts_ctl(&mut self, value: HsfStsCtl) {
        self.hsfsts_ctl.write(value.bits());
    }

    pub fn fdo(&mut self, section: FdoSection, index: u16) -> u32 {
        self.fdoc.write(
            (section as u32) |
            (((index & 0b1111111111) as u32) << 2)
        );
        self.fdod.read()
    }
}

impl Spi for SpiRegs {
    fn len(&mut self) -> Result<usize, SpiError> {
        let kib = 1024;
        let mib = 1024 * kib;

        let component = self.fdo(FdoSection::Component, 0);
        Ok(match component & 0b111 {
            0b000 => 512 * kib,
            0b001 => mib,
            0b010 => 2 * mib,
            0b011 => 4 * mib,
            0b100 => 8 * mib,
            0b101 => 16 * mib,
            0b110 => 32 * mib,
            0b111 => 64 * mib,
            _ => return Err(SpiError::Register)
        })
    }

    fn read(&mut self, address: usize, buf: &mut [u8]) -> Result<usize, SpiError> {
        let mut count = 0;
        for chunk in buf.chunks_mut(64) {
            let mut hsfsts_ctl;

            // Wait for other transactions
            loop {
                hsfsts_ctl = self.hsfsts_ctl();
                if ! hsfsts_ctl.contains(HsfStsCtl::H_SCIP) {
                    break;
                }
            }

            hsfsts_ctl.sanitize();
            self.set_hsfsts_ctl(hsfsts_ctl);

            hsfsts_ctl.set_cycle(HsfStsCtlCycle::Read);
            hsfsts_ctl.set_count(chunk.len() as u8);
            hsfsts_ctl.insert(HsfStsCtl::FGO);

            // Start command
            self.faddr.write((address + count) as u32);
            self.set_hsfsts_ctl(hsfsts_ctl);

            // Wait for command to finish
            loop {
                hsfsts_ctl = self.hsfsts_ctl();

                if hsfsts_ctl.contains(HsfStsCtl::FCERR) {
                    hsfsts_ctl.sanitize();
                    self.set_hsfsts_ctl(hsfsts_ctl);

                    return Err(SpiError::Cycle);
                }

                if hsfsts_ctl.contains(HsfStsCtl::FDONE) {
                    break;
                }
            }

            for (i, dword) in chunk.chunks_mut(4).enumerate() {
                let data = self.fdata[i].read();
                for (j, byte) in dword.iter_mut().enumerate() {
                    *byte = (data >> (j * 8)) as u8;
                }
            }

            hsfsts_ctl.sanitize();
            self.set_hsfsts_ctl(hsfsts_ctl);

            count += chunk.len()
        }
        Ok(count)
    }

    fn erase(&mut self, address: usize) -> Result<(), SpiError> {
        let mut hsfsts_ctl;

        // Wait for other transactions
        loop {
            hsfsts_ctl = self.hsfsts_ctl();
            if ! hsfsts_ctl.contains(HsfStsCtl::H_SCIP) {
                break;
            }
        }

        hsfsts_ctl.sanitize();
        self.set_hsfsts_ctl(hsfsts_ctl);

        hsfsts_ctl.set_cycle(HsfStsCtlCycle::BlockErase);
        hsfsts_ctl.insert(HsfStsCtl::FGO);

        // Start command
        self.faddr.write(address as u32);
        self.set_hsfsts_ctl(hsfsts_ctl);

        // Wait for command to finish
        loop {
            hsfsts_ctl = self.hsfsts_ctl();

            if hsfsts_ctl.contains(HsfStsCtl::FCERR) {
                hsfsts_ctl.sanitize();
                self.set_hsfsts_ctl(hsfsts_ctl);

                return Err(SpiError::Cycle);
            }

            if hsfsts_ctl.contains(HsfStsCtl::FDONE) {
                break;
            }
        }

        hsfsts_ctl.sanitize();
        self.set_hsfsts_ctl(hsfsts_ctl);

        Ok(())
    }

    fn write(&mut self, address: usize, buf: &[u8]) -> Result<usize, SpiError> {
        let mut count = 0;
        for chunk in buf.chunks(64) {
            let mut hsfsts_ctl;

            // Wait for other transactions
            loop {
                hsfsts_ctl = self.hsfsts_ctl();
                if ! hsfsts_ctl.contains(HsfStsCtl::H_SCIP) {
                    break;
                }
            }

            hsfsts_ctl.sanitize();
            self.set_hsfsts_ctl(hsfsts_ctl);

            hsfsts_ctl.set_cycle(HsfStsCtlCycle::Write);
            hsfsts_ctl.set_count(chunk.len() as u8);
            hsfsts_ctl.insert(HsfStsCtl::FGO);

            // Fill data
            for (i, dword) in chunk.chunks(4).enumerate() {
                let mut data = 0;
                for (j, byte) in dword.iter().enumerate() {
                    data |= (*byte as u32) << (j * 8);
                }
                self.fdata[i].write(data);
            }

            // Start command
            self.faddr.write((address + count) as u32);
            self.set_hsfsts_ctl(hsfsts_ctl);

            // Wait for command to finish
            loop {
                hsfsts_ctl = self.hsfsts_ctl();

                if hsfsts_ctl.contains(HsfStsCtl::FCERR) {
                    hsfsts_ctl.sanitize();
                    self.set_hsfsts_ctl(hsfsts_ctl);

                    return Err(SpiError::Cycle);
                }

                if hsfsts_ctl.contains(HsfStsCtl::FDONE) {
                    break;
                }
            }

            hsfsts_ctl.sanitize();
            self.set_hsfsts_ctl(hsfsts_ctl);

            count += chunk.len()
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::SpiRegs;

    #[test]
    fn offsets() {
        unsafe {
            let spi: &SpiRegs = &*(0 as *const SpiRegs);

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
