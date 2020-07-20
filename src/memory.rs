use memmap::Mmap;

pub struct Memory {
    rom: Mmap,
    pub data: [u8; 65536],
}

impl Memory {
    pub fn with_rom(rom: Mmap) -> Memory {
        Memory {
            rom: rom,
            data: [0; 65536],
        }
    }

    pub fn read8(&self, addr: &u16) -> u8 {
        let mut offset: u16 = 0;

        // RAM echo
        if 0xe000 <= *addr && *addr < 0xfe00 {
            offset = 0x1000;
        }

        self.data[(*addr - offset) as usize]
    }
    
    pub fn read16(&self, addr: &u16) -> u16 {
        let top: u16 = self.data[*addr as usize] as u16;
        let bottom: u16 = (self.data[(*addr + 1) as usize] as u16) << 8;
        bottom | top
    }

    pub fn write8(&self, addr: u16, val: u8) {
        // TODO
    }

    pub fn write16(&self, addr: u16, val: u16) {
        self.write8(addr, (val & 0xff) as u8);
        self.write8(addr + 1, (val >> 8) as u8);
    }

    pub fn mem_dma(&mut self, addr: u16) {
        let start = addr as usize;
        let end = start + 160;
        self.data.copy_within(start..end, 0xfe00)
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

}