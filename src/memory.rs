use crate::rom::{CartridgeController, ROM, ROMSize};
use crate::memory_types::{IORegisters, Joypad, PPUMode};

pub struct Memory<'a> {
    pub rom: ROM<'a>,
    pub data: [u8; 65536],
    pub joypad_states: [u8; 2],
    memory_bank: usize,
    external_ram_enabled: bool,
}

// IORegisters is an exact byte-for-byte view of $FE00-$FFFF.
const _: [(); 0x200] = [(); std::mem::size_of::<IORegisters>()];
const _: [(); 1] = [(); std::mem::align_of::<IORegisters>()];

impl Memory<'_> {
    pub fn with_rom_buffer(rom_buffer: &[u8]) -> Memory<'_> {
        let rom = ROM::new(rom_buffer);
        let external_ram_enabled = matches!(
            rom.cartridge_controller(),
            CartridgeController::NoMBC { has_ram: true },
        );
        let mut memory = Memory {
            rom,
            data: [0; 65536],
            joypad_states: [0, 0],
            memory_bank: 1,
            external_ram_enabled,
        };
        memory.reg_mut().lcd_stat.set_ppu_mode(PPUMode::VBlank);
        memory.reg_mut().lcd_stat.set_lyc_equal(true);
        memory
    }

    pub fn reg(&self) -> &IORegisters {
        unsafe { &*(self.data[0xfe00..].as_ptr() as *const IORegisters) }
    }

    pub fn reg_mut(&mut self) -> &mut IORegisters {
        unsafe { &mut *(self.data[0xfe00..].as_mut_ptr() as *mut IORegisters) }
    }

    pub fn read8(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3fff => {
                // rom bank 0
                self.rom.buffer[address as usize]
            },
            0x4000..=0x7fff => {
                // TODO: switchable ROM bank
                let adjusted_address = 0x4000 * (self.memory_bank - 1) + (address as usize);
                self.rom.buffer[adjusted_address]
            },
            0x8000..=0x9fff => {
                // vram
                self.data[address as usize]
            },
            0xa000..=0xbfff => {
                self.read_external_ram(address)
            },
            0xc000..=0xcfff => {
                // work ram
                self.data[address as usize]
            },
            0xd000..=0xdfff => {
                // work ram
                self.data[address as usize]
            },
            0xe000..=0xfdff => {
                // echo ram, mirror of $c000–$ddff
                self.data[(address - 0x1000) as usize]
            },
            0xfe00..=0xffff => {
                self.data[address as usize]
            },
        }
    }

    pub fn read16(&self, addr: u16) -> u16 {
        let top: u16 = self.read8(addr) as u16;
        let bottom: u16 = (self.read8(addr + 1) as u16) << 8;
        bottom | top
    }

    pub fn write8(&mut self, addr: u16, val: u8) {
        // WIP
        match addr {
            0x0000..=0x1fff => {
                self.set_external_ram_enabled(addr, val);
            },
            0x2000..=0x3fff => {
                // TODO: implement all ROM sizes
                match self.rom.header.rom_size {
                    ROMSize::BANKS2 => {
                        // noop
                    },
                    ROMSize::BANKS64 => {
                        match addr {
                            0x2000..=0x2fff => {
                                self.memory_bank &= !0xff;
                                self.memory_bank |= val as usize;
                            },
                            0x3000..=0x3fff => {
                                self.memory_bank &= !0x100;
                                self.memory_bank |= ((val & 0x1) as usize) << 8;
                            },
                            _ => {},
                        }
                    },
                    _ => { panic!("not supported yet!") }
                }
            },
            0x4000..=0x5fff => {
                // TODO: ???
            },
            0x6000..=0x7fff => {
                // TODO: actually switch memory mode
                // memory mode select
                // MEMORY_MODE = val & 1;
            },
            0x8000..=0x9fff => {
                // video RAM
                self.data[addr as usize] = val;
            },
            0xa000..=0xbfff => {
                self.write_external_ram(addr, val);
            },
            0xc000..=0xdfff => {
                // low RAM
                self.data[addr as usize] = val;
            },
            // 0xfe00..=0xfebf => {
            //     // OAM
            //     self.data[addr as usize] = val;
            // },
            0xfea0..=0xfeff => {
                // empty ???
            },
            0xff00 => {
                // joypad, only top nibble is writable
                match Joypad::from(val) {
                    Joypad::Both => {
                        self.reg_mut().joypad = self.joypad_states[0] & self.joypad_states[1];
                    },
                    Joypad::Buttons => {
                        self.reg_mut().joypad = Joypad::Buttons as u8 | self.joypad_states[0];
                    },
                    Joypad::Directional => {
                        self.reg_mut().joypad = Joypad::Directional as u8 | self.joypad_states[1];
                    },
                    Joypad::None => {
                        self.reg_mut().joypad = 0x3f;
                    },
                }
            },
            0xff46 => {
                // dma
                self.reg_mut().oam_dma_source_address = val;
                self.mem_dma((val as u16) << 8);
            },
            0xfe00..=0xffff => {
                self.data[addr as usize] = val;
            },
            _ => {
                panic!("Unhandled memory write to address: 0x{:04X}", addr);
            }
        }
    }

    pub fn write16(&mut self, addr: u16, val: u16) {
        self.write8(addr, (val & 0xff) as u8);
        self.write8(addr + 1, (val >> 8) as u8);
    }

    pub fn mem_dma(&mut self, addr: u16) {
        let start = addr as usize;
        let end = start + 160;
        let source = self.data[start..end].to_owned();
        self.reg_mut().sprites.copy_from_slice(&source);
    }

    fn set_external_ram_enabled(&mut self, address: u16, value: u8) {
        let accepts_write = match self.rom.cartridge_controller() {
            CartridgeController::MBC1 { .. }
            | CartridgeController::MBC3 { .. }
            | CartridgeController::MBC5 { .. } => true,
            CartridgeController::MBC2 => address & 0x0100 == 0,
            _ => false,
        };

        if accepts_write {
            self.external_ram_enabled = value & 0x0f == 0x0a;
        }
    }

    fn external_ram_index(&self, address: u16) -> Option<usize> {
        match self.rom.cartridge_controller() {
            CartridgeController::NoMBC { has_ram: true }
            | CartridgeController::MBC1 { has_ram: true }
            | CartridgeController::MBC3 { has_ram: true, .. }
            | CartridgeController::MBC5 { has_ram: true, .. } => Some(address as usize),
            CartridgeController::MBC2 => {
                Some(0xa000 + ((address as usize - 0xa000) & 0x01ff))
            },
            _ => None,
        }
    }

    fn read_external_ram(&self, address: u16) -> u8 {
        if !self.external_ram_enabled {
            return 0xff;
        }

        match self.external_ram_index(address) {
            Some(index) if self.rom.cartridge_controller() == CartridgeController::MBC2 => {
                0xf0 | self.data[index]
            },
            Some(index) => self.data[index],
            None => 0xff,
        }
    }

    fn write_external_ram(&mut self, address: u16, value: u8) {
        if !self.external_ram_enabled {
            return;
        }

        if let Some(index) = self.external_ram_index(address) {
            self.data[index] = if self.rom.cartridge_controller() == CartridgeController::MBC2 {
                value & 0x0f
            } else {
                value
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Memory;

    fn rom_with_cartridge_type(cartridge_type: u8) -> [u8; 0x150] {
        let mut rom = [0; 0x150];
        rom[0x147] = cartridge_type;
        rom
    }

    #[test]
    fn selecting_both_joypad_groups_combines_active_low_inputs() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        memory.joypad_states = [0b1110, 0b1101];

        memory.write8(0xff00, 0x00);

        assert_eq!(memory.reg().joypad, 0b1100);
    }

    #[test]
    fn register_view_shares_the_data_backing_array() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);

        memory.reg_mut().timer_tima = 0x42;
        assert_eq!(memory.data[0xff05], 0x42);

        memory.data[0xff06] = 0x99;
        assert_eq!(memory.reg().timer_tma, 0x99);
    }

    #[test]
    fn unbanked_external_ram_is_always_accessible() {
        let rom = rom_with_cartridge_type(0x08);
        let mut memory = Memory::with_rom_buffer(&rom);

        memory.write8(0xa123, 0x42);
        memory.write8(0x0000, 0x00);

        assert_eq!(memory.read8(0xa123), 0x42);
    }

    #[test]
    fn mbc_external_ram_requires_enablement() {
        for cartridge_type in [0x03, 0x10, 0x1e].iter().copied() {
            let rom = rom_with_cartridge_type(cartridge_type);
            let mut memory = Memory::with_rom_buffer(&rom);

            memory.write8(0xa123, 0x11);
            assert_eq!(memory.read8(0xa123), 0xff);

            memory.write8(0x0000, 0x1a);
            memory.write8(0xa123, 0x42);
            assert_eq!(memory.read8(0xa123), 0x42);

            memory.write8(0x0000, 0x00);
            assert_eq!(memory.read8(0xa123), 0xff);

            memory.write8(0x0000, 0x0a);
            assert_eq!(memory.read8(0xa123), 0x42);
        }
    }

    #[test]
    fn cartridges_without_ram_keep_external_space_unmapped() {
        let rom = rom_with_cartridge_type(0x01);
        let mut memory = Memory::with_rom_buffer(&rom);

        memory.write8(0x0000, 0x0a);
        memory.write8(0xa000, 0x42);

        assert_eq!(memory.read8(0xa000), 0xff);
    }

    #[test]
    fn mbc2_uses_address_bit_eight_and_four_bit_ram() {
        let rom = rom_with_cartridge_type(0x05);
        let mut memory = Memory::with_rom_buffer(&rom);

        memory.write8(0x0100, 0x0a);
        memory.write8(0xa000, 0xab);
        assert_eq!(memory.read8(0xa000), 0xff);

        memory.write8(0x0000, 0x0a);
        memory.write8(0xa000, 0xab);
        assert_eq!(memory.read8(0xa000), 0xfb);
        assert_eq!(memory.read8(0xa200), 0xfb);

        memory.write8(0x0000, 0x00);
        assert_eq!(memory.read8(0xa000), 0xff);
    }
}
