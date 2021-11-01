// SPDX-License-Identifier: MIT

extern crate coreboot_fs;
extern crate libc;
extern crate intel_spi;

use coreboot_fs::Rom;
use intel_spi::Spi;
use std::collections::BTreeMap;
use std::{env, fs, process};

#[path = "../examples/util/mod.rs"]
mod util;

fn copy_region(region: intelflash::RegionKind, old_data: &[u8], new_data: &mut [u8]) -> Result<bool, String> {
    let old_opt = intelflash::Rom::new(old_data)?.get_region_base_limit(region)?;
    let new_opt = intelflash::Rom::new(new_data)?.get_region_base_limit(region)?;

    if old_opt.is_none() && new_opt.is_none() {
        // Neither ROM has this region, so ignore it
        return Ok(false);
    }

    let old = match old_opt {
        Some((base, limit)) => if base < limit && limit < old_data.len() {
            &old_data[base..limit + 1]
        } else {
            return Err(format!("old region {:#X}:{:#X} is invalid", base, limit));
        },
        None => return Err(format!("missing old region")),
    };

    let new = match new_opt {
        Some((base, limit)) => if base < limit && limit < new_data.len() {
            &mut new_data[base..limit + 1]
        } else {
            return Err(format!("new region {:#X}:{:#X} is invalid", base, limit));
        },
        None => return Err(format!("missing new region")),
    };

    if old.len() != new.len() {
        return Err(format!("old region size {} does not match new region size {}", old.len(), new.len()));
    }

    new.copy_from_slice(old);
    Ok(true)
}

fn main() {
    let path = match env::args().nth(1) {
        Some(some) => some,
        None => {
            eprintln!("intel-spi [rom file]");
            process::exit(1);
        }
    };

    let spi = unsafe { util::get_spi() };

    eprintln!("SPI HSFSTS_CTL: {:?}", spi.hsfsts_ctl());

    // Read new data
    let mut new;
    {
        let loading = "Loading";
        eprint!("SPI FILE: {}", loading);
        new = fs::read(path).unwrap();
        for _c in loading.chars() {
            eprint!("\x08 \x08");
        }
        eprintln!("{} MB", new.len() / (1024 * 1024));
    }

    // Grab new FMAP areas area, if they exist
    let mut new_areas = BTreeMap::new();
    {
        let rom = Rom::new(&new);
        if let Some(fmap) = rom.fmap() {
            let name: String = fmap.name.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();

            eprintln!("  {}", name);

            for i in 0..fmap.nareas {
                let area = fmap.area(i);

                let name: String = area.name.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();

                eprintln!("    {}: {}", i, name);

                new_areas.insert(name, *area);
            }
        }
    }

    // Check ROM size
    let len = spi.len().unwrap();
    eprintln!("SPI ROM: {} MB", len / (1024 * 1024));
    assert!(len == new.len(), "firmware.rom size invalid");

    // Read current data
    let mut data;
    {
        data = Vec::with_capacity(len);
        let mut print_mb = !0; // Invalid number to force first print
        while data.len() < len {
            let mut buf = [0; 4096];
            let read = spi.read(data.len(), &mut buf).unwrap();
            data.extend_from_slice(&buf[..read]);

            // Print output once per megabyte
            let mb = data.len() / (1024 * 1024);
            if mb != print_mb {
                eprint!("\rSPI READ: {} MB", mb);
                print_mb = mb;
            }
        }
        eprintln!();
    }

    // Copy GBE region, if it exists
    match copy_region(intelflash::RegionKind::Ethernet, &data, &mut new) {
        Ok(true) => eprintln!("Ethernet: copied region from old firmware to new firmare"),
        Ok(false) => (),
        Err(err) => eprintln!("Ethernet: failed to copy: {}", err),
    }

    // Grab old FMAP areas, if they exist
    let mut areas = BTreeMap::new();
    {
        let rom = Rom::new(&data);
        if let Some(fmap) = rom.fmap() {
            let name: String = fmap.name.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();

            eprintln!("  {}", name);

            for i in 0..fmap.nareas {
                let area = fmap.area(i);

                let name: String = area.name.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();

                eprintln!("    {}: {}", i, name);

                areas.insert(name, *area);
            }
        }
    }

    // Copy old areas to new areas
    let area_names: &[String] = &[
        //Warning: Copying these regions can be dangerous
        // "RW_MRC_CACHE".to_string(),
        // "SMMSTORE".to_string(),
    ];
    for area_name in area_names {
        if let Some(new_area) = new_areas.get(area_name) {
            let new_offset = new_area.offset as usize;
            let new_size = new_area.size as usize;
            eprintln!(
                "{}: found in new firmware: offset {:#X}, size {} KB",
                area_name,
                new_offset,
                new_size / 1024
            );
            let new_slice = new.get_mut(
                new_offset .. new_offset + new_size
            ).unwrap();

            if let Some(area) = areas.get(area_name) {
                let offset = area.offset as usize;
                let size = area.size as usize;
                eprintln!(
                    "{}: found in old firmware: offset {:#X}, size {} KB",
                    area_name,
                    new_offset,
                    new_size / 1024
                );
                let slice = data.get(
                    offset .. offset + size
                ).unwrap();

                if slice.len() == new_slice.len() {
                    new_slice.copy_from_slice(slice);

                    eprintln!(
                        "{}: copied from old firmware to new firmware",
                        area_name
                    );
                } else {
                    eprintln!(
                        "{}: old firmware size {} does not match new firmware size {}, not copying",
                        area_name,
                        slice.len(),
                        new_slice.len()
                    );
                }
            } else {
                eprintln!(
                    "{}: found in new firmware, but not found in old firmware",
                    area_name
                );
            }
        } else if areas.get(area_name).is_some() {
            eprintln!(
                "{}: found in old firmware, but not found in new firmware",
                area_name
            );
        }
    }

    // Erase and write
    {
        let erase_byte = 0xFF;
        let erase_size = 4096;
        let mut i = 0;
        let mut print_mb = !0; // Invalid number to force first print
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
                    spi.write(i, new_chunk).unwrap();
                }
            }

            i += chunk.len();

            // Print output once per megabyte
            let mb = i / (1024 * 1024);
            if mb != print_mb {
                eprint!("\rSPI WRITE: {} MB", mb);
                print_mb = mb;
            }
        }
        eprintln!();
    }

    // Verify
    {
        data.clear();
        let mut print_mb = !0; // Invalid number to force first print
        while data.len() < len {
            let mut address = data.len();

            let mut buf = [0; 4096];
            let read = spi.read(address, &mut buf).unwrap();
            data.extend_from_slice(&buf[..read]);

            while address < data.len() {
                assert!(data[address] == new[address],
                    "\nverification failed as {:#x}: {:#x} != {:#x}",
                    address,
                    data[address],
                    new[address]
                );

                address += 1;
            }

            let mb = data.len() / (1024 * 1024);
            if mb != print_mb {
                eprint!("\rSPI VERIFY: {} MB", mb);
                print_mb = mb;
            }
        }
        eprintln!();
    }

    unsafe { util::release_spi(spi); }
}
