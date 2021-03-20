use memmap::Mmap;

pub struct Memory {
    rom: Mmap,
    pub data: [u8; 65536],
    pub joypad_states: [u8; 2],
}

impl Memory {
    pub fn with_rom(rom: Mmap) -> Memory {
        Memory {
            rom: rom,
            data: [0; 65536],
            joypad_states: [0, 0],
        }
    }

    pub fn read8(&self, addr: u16) -> u8 {
        let mut offset: u16 = 0;

        if addr < 0x4000 {
            return self.rom[addr as usize];
        }

        if addr < 0x8000 {
            // TODO switchable ROM bank
            return self.rom[addr as usize];
        }

        // RAM echo
        if 0xe000 <= addr && addr < 0xfe00 {
            offset = 0x1000;
        }

        self.data[(addr - offset) as usize]
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
                // TODO: ???
            },
            0x2000..=0x3fff => {
                // ROM bank select (0 points to one as well)
                let bank = if val == 0 { 1 } else { val };
                println!("Switching to ROM bank {}\n", bank);
                // TODO: actually switch banks
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
                // switchable RAM bank
            },
            0xc000..=0xdfff => {
                // low RAM
                self.data[addr as usize] = val;
            },
            0xfe00..=0xfebf => {
                // OAM
                self.data[addr as usize] = val;
            },
            0xfea0..=0xfeff => {
                // empty ???
            },
            0xff00 => {
                // joypad
                if val & 0x10 != 0 {
                    // non-directional
                    *self.reg_joypad() = 0xd0 | self.joypad_states[0];
                } else if val & 0x20 != 0 {
                    // directional
                    *self.reg_joypad() = 0xe0 | self.joypad_states[1];
                }
            },
            0xff04 => {
                // interrupt register
                self.data[addr as usize] = val;
            },
            0xff0f => {
                // divider register
                *self.reg_div() = 0;
            },
            0xff40 => {
                // lcdc
                *self.reg_lcdc() = val;
                // ???
            },
            0xff41 => {
                // lcdc stat
                *self.reg_stat() = val;
            },
            0xff46 => {
                // dma
                *self.reg_dma() = val;
                self.mem_dma((val as u16) << 8);
            },
            0xff00..=0xff7f => {
                // IO ports + empty
                self.data[addr as usize] = val;
            },
            0xff80..=0xfffe => {
                // internal RAM
                self.data[addr as usize] = val;
            },
            0xffff => {
                // interrupt enable register
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
        self.data.copy_within(start..end, 0xfe00)
    }
    // Memory Mapped IO

    pub fn reg_joypad(&mut self) -> &mut u8 {
        &mut self.data[0xff00]
    }

    pub fn reg_div(&mut self) -> &mut u8 {
        &mut self.data[0xff04]
    }

    pub fn reg_tima(&mut self) -> &mut u8 {
        &mut self.data[0xff05]
    }

    pub fn reg_tma(&mut self) -> &mut u8 {
        &mut self.data[0xff06]
    }

    pub fn reg_tac(&mut self) -> &mut u8 {
        &mut self.data[0xff07]
    }

    pub fn reg_lcdc(&mut self) -> &mut u8 {
        &mut self.data[0xff40]
    }

    pub fn reg_stat(&mut self) -> &mut u8 {
        &mut self.data[0xff41]
    }

    pub fn reg_scy(&mut self) -> &mut u8 {
        &mut self.data[0xff42]
    }

    pub fn reg_scx(&mut self) -> &mut u8 {
        &mut self.data[0xff43]
    }

    pub fn reg_ly(&mut self) -> &mut u8 {
        &mut self.data[0xff44]
    }

    pub fn reg_dma(&mut self) -> &mut u8 {
        &mut self.data[0xff47]
    }

    pub fn reg_wy(&mut self) -> &mut u8 {
        &mut self.data[0xff4a]
    }

    pub fn reg_wx(&mut self) -> &mut u8 {
        &mut self.data[0xff4b]
    }
}
