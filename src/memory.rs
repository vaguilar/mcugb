use memmap::Mmap;

enum ROMSize {
    BANKS2,
    // BANKS4,
    // BANKS8,
    // BANKS16,
    // BANKS32,
    BANKS64,
    // BANKS128,
    // BANKS256,
    // BANKS512,
}

pub struct Memory {
    rom: Mmap,
    pub data: [u8; 65536],
    pub joypad_states: [u8; 2],
    rom_size: ROMSize,
    memory_bank: usize,
}

impl Memory {
    pub fn with_rom(rom: Mmap) -> Memory {
        let rom_size: ROMSize = match rom[0x0148] {
            0 => ROMSize::BANKS2,
            5 => ROMSize::BANKS64,
            n => panic!("Unhandled ROM size {}", n),
        };
        Memory {
            rom,
            data: [0; 65536],
            joypad_states: [0, 0],
            rom_size,
            memory_bank: 1,
        }
    }

    pub fn read8(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3fff => self.rom[address as usize],
            0x4000..=0x7fff => {
                // TODO switchable ROM bank
                let adjusted_address = 0x4000 * (self.memory_bank - 1) + (address as usize);
                self.rom[adjusted_address]
            },
            0xe000..=0xfdff => {
                self.data[(address - 0x1000) as usize]
            },
            0xff41 => {
                // TODO
                let ly = self.data[0xff44];
                let lyc = self.data[0xff45];
                0x80 | if ly == lyc { 2 } else { 0 }
            }
            _ => self.data[address as usize]
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
                if val < 2 {
                    return;
                }
                // TODO: ??? enable RAM bank?
                panic!("Unhandled write to 0x0000..=0x1fff, val = {:}", val);
            },
            0x2000..=0x3fff => {
                // TODO: implement all ROM sizes
                match self.rom_size {
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
        self.data.copy_within(start..end, 0xfe00) // TODO use copy_nonoverlapping?
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

    // pub fn reg_lyc(&mut self) -> &mut u8 {
    //     &mut self.data[0xff45]
    // }

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

/// memory starting at start at $fe00
#[repr(C)]
struct IORegisters {
    /// $fe00
    pub sprites: [u8; 160],
    /// Nintendo says use of this area is prohibited
    _prohibited: [u8; 96],

    // io registers
    /// $ff00
    pub joypad: u8,
    /// $ff01
    pub serial_transfer: [u8; 2],
    _padding0: u8,
    /// $ff04
    pub timer_divider: [u8; 4],
    _padding1: [u8; 7],
    /// $ff0f
    pub interrupts: u8,
    /// $ff10
    pub audio: [u8; 22],
    _padding2: [u8; 10],
    /// $ff30
    pub wave_pattern: [u8; 16],
    /// $ff40
    pub lcd_control: u8,
    /// $ff41
    pub lcd_stat: u8,
    /// $ff42
    pub lcd_scx: u8,
    /// $ff43
    pub lcd_scy: u8,
    /// $ff44: current scan line -- READ-ONLY
    pub lcd_y: u8,
    /// $ff45
    pub lcd_yc: u8,
    /// $ff46
    pub oam_dma_source_address: u8,
    /// $ff47
    pub bg_palette_data: u8,
    /// $ff48
    pub object_palette_0: u8,
    /// $ff49
    pub object_palette_1: u8,
    /// $ff4a
    pub wx: u8,
    /// $ff4b
    pub wy: u8,
    _padding3: [u8; 3],
    /// $ff4f
    pub vram_bank_select: u8,
    /// $ff50
    pub bootrom_disable: u8,
    /// $ff51
    pub vram_dma: [u8; 5],
    _padding4: [u8; 18],
    /// $ff68
    pub bg_object_pallets: [u8; 4],
    _padding5: [u8; 4],
    /// $ff70
    pub wram_bank_select: u8,
    _padding6: [u8; 15],

    /// high ram $ff80
    pub hram: [u8; 0x7f],
    /// $ffff
    pub interrupt_enable: u8,
}

impl IORegisters {
    fn new() -> IORegisters {
        unsafe { std::mem::zeroed() }
    }
}