// SPDX-License-Identifier: MIT

use core::ops::{BitAnd, BitOr, Not};
use core::ptr;

use super::io::Io;

#[repr(transparent)]
pub struct Mmio<T> {
    value: T
}

impl<T> Io for Mmio<T> where T: Copy + PartialEq + BitAnd<Output = T> + BitOr<Output = T> + Not<Output = T> {
    type Value = T;

    fn read(&self) -> T {
        unsafe { ptr::read_volatile(&self.value) }
    }

    fn write(&mut self, value: T) {
        unsafe { ptr::write_volatile(&mut self.value, value) };
    }
}
