#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PhysicalAddress(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct VirtualAddress(pub usize);

pub trait Mapper {
    unsafe fn map_aligned(&mut self, address: PhysicalAddress, size: usize) -> Result<VirtualAddress, &'static str>;
    unsafe fn unmap_aligned(&mut self, address: VirtualAddress, size: usize) -> Result<(), &'static str>;
    fn page_size(&self) -> usize;

    unsafe fn map(&mut self, address: PhysicalAddress, size: usize) -> Result<VirtualAddress, &'static str> {
        let page_size = self.page_size();
        let page = address.0/page_size;
        let aligned_address = PhysicalAddress(page * page_size);
        let offset = address.0 - aligned_address.0;
        let pages = (offset + size + page_size - 1) / page_size;
        let aligned_size = pages * page_size;
        let virtual_address = self.map_aligned(aligned_address, aligned_size)?;
        Ok(VirtualAddress(virtual_address.0 + offset))
    }

    unsafe fn unmap(&mut self, address: VirtualAddress, size: usize) -> Result<(), &'static str> {
        let page_size = self.page_size();
        let page = address.0/page_size;
        let aligned_address = VirtualAddress(page * page_size);
        let offset = address.0 - aligned_address.0;
        let pages = (offset + size + page_size - 1) / page_size;
        let aligned_size = pages * page_size;
        self.unmap_aligned(aligned_address, aligned_size)
    }
}
