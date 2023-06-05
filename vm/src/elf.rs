// Copyright 2023 MOZAK.

use anyhow::{anyhow, bail, Result};
use elf::{endian::LittleEndian, file::Class, ElfBytes};
use im::hashmap::HashMap;
use itertools::Itertools;

/// A RISC program
pub struct Program {
    /// The entrypoint of the program
    pub entry: u32,

    /// The initial memory image
    pub image: HashMap<u32, u8>,
}

impl From<HashMap<u32, u8>> for Program {
    fn from(image: HashMap<u32, u8>) -> Self {
        Self {
            entry: 0_u32,
            image,
        }
    }
}

#[cfg(test)]
impl From<HashMap<u32, u32>> for Program {
    fn from(image: HashMap<u32, u32>) -> Self {
        let image = image
            .iter()
            .flat_map(move |(k, v)| {
                v.to_le_bytes()
                    .into_iter()
                    .enumerate()
                    .map(move |(i, b)| (k + i as u32, b))
            })
            .collect();
        Self {
            entry: 0_u32,
            image,
        }
    }
}

impl Program {
    /// Initialize a RISC Program from an appropriate ELF file
    ///
    /// # Errors
    /// Will return `Err` if the ELF file is invalid or if the entrypoint is
    /// invalid.
    pub fn load_elf(input: &[u8]) -> Result<Program> {
        let elf = ElfBytes::<LittleEndian>::minimal_parse(input)?;
        if elf.ehdr.class != Class::ELF32 {
            bail!("Not a 32-bit ELF");
        }
        if elf.ehdr.e_machine != elf::abi::EM_RISCV {
            bail!("Invalid machine type, must be RISC-V");
        }
        if elf.ehdr.e_type != elf::abi::ET_EXEC {
            bail!("Invalid ELF type, must be executable");
        }
        let entry: u32 = elf.ehdr.e_entry.try_into()?;
        if entry % 4 != 0 {
            bail!("Invalid entrypoint");
        }
        let segments = elf
            .segments()
            .ok_or_else(|| anyhow!("Missing segment table"))?;
        if segments.len() > 256 {
            bail!("Too many program headers");
        }

        let image = segments
            .iter()
            .filter(|x| x.p_type == elf::abi::PT_LOAD)
            .map(|segment| -> Result<_> {
                let file_size: usize = segment.p_filesz.try_into()?;
                let mem_size: usize = segment.p_memsz.try_into()?;
                let vaddr: u32 = segment.p_vaddr.try_into()?;
                let offset = segment.p_offset.try_into()?;
                Ok(input[offset..offset + std::cmp::min(file_size, mem_size)]
                    .iter()
                    .enumerate()
                    .map(move |(i, b)| (vaddr + i as u32, *b)))
            })
            .flatten_ok()
            .try_collect()?;
        Ok(Program { entry, image })
    }
}
