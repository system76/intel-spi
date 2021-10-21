// SPDX-License-Identifier: MIT

use core::ops::{BitAnd, BitOr, Not};
use core::ptr;

use super::io::Io;

#[repr(packed)]
pub struct Mmio<T> {
    value: T
}

impl<T> Io for Mmio<T> where T: Copy + PartialEq + BitAnd<Output = T> + BitOr<Output = T> + Not<Output = T> {
    type Value = T;

    fn read(&self) -> T {
        let raw = ptr::addr_of!(self.value);
        unsafe { raw.read_volatile() }
    }

    fn write(&mut self, value: T) {
        let raw = ptr::addr_of_mut!(self.value);
        unsafe { raw.write_volatile(value) };
    }
}
