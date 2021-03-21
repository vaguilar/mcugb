use crate::memory::Memory;
use std::convert::TryInto;

// static LCDC_ON: u8 = 1 << 7;
static LCDC_WINDOW_TILE_MAP_SELECT: u8 = 1 << 6;
static LCDC_WINDOW_ON: u8 = 1 << 5;
static LCDC_BG_TILE_DATA: u8 = 1 << 4;
static LCDC_BG_TILE_MAP_SELECT: u8 = 1 << 3;
static LCDC_SPRITE_DOUBLE_HEIGHT: u8 = 1 << 2;
static LCDC_SHOW_SPRITES: u8 = 1 << 1;
// static LCDC_SHOW_BG: u8 = 1 << 0;


// static SPRITE_PRIORITY: u8 = 1 << 7;
static SPRITE_FLIP_V:u8 = 1 << 6;
static SPRITE_FLIP_H: u8 = 1 << 5;

static COLORS: [(u8, u8); 4] = [(0xe7, 0x9c), (0x97, 0x08), (0x44, 0x31), (0x31, 0x6a)];

pub struct GPU {
    pub clock: u16,
    pub mode: u8,
}

impl GPU {
    pub fn step(&mut self, mem: &mut Memory, cycles: u16) -> (bool, bool) {
        let mut redraw = false;
        let mut vblank = false;
        self.clock += cycles;

        match self.mode {
            0 => {
                // HBlank
                if self.clock >= 204 {
                    self.clock = 0;
                    *mem.reg_ly() = mem.reg_ly().wrapping_add(1);

                    if *mem.reg_ly() == 143 {
                        self.mode = 1;
                        *mem.reg_stat() = (*mem.reg_stat() & 0xfc) | self.mode;
                        vblank = true;
                        redraw = true;
                    } else {
                        self.mode = 2;
                        *mem.reg_stat() = (*mem.reg_stat() & 0xfc) | self.mode;
                    }
                }
            }
            1 => {
                // VBlank
                if self.clock >= 456 {
                    self.clock = 0;
                    *mem.reg_ly() = mem.reg_ly().wrapping_add(1);

                    if *mem.reg_ly() > 153 {
                        *mem.reg_ly() = 0;
                        self.mode = 2;
                        *mem.reg_stat() = (*mem.reg_stat() & 0xfc) | self.mode;
                    }
                }
            }
            2 => {
                // OAM read mode
                if self.clock >= 80 {
                    self.clock = 0;
                    self.mode = 3;
                    *mem.reg_stat() = (*mem.reg_stat() & 0xfc) | self.mode;
                }
            }
            3 => {
                // VRAM read mode
                if self.clock >= 172 {
                    self.clock = 0;
                    self.mode = 0;
                    *mem.reg_stat() = (*mem.reg_stat() & 0xfc) | self.mode;
                    // self.draw_scanline();
                }
            }
            _ => {
                panic!("Unhandled GPU mode {}", self.mode)
            }
        }
        (redraw, vblank)
    }

    #[inline]
    fn set_pixel(buffer: &mut [u8], x: u8, y: u8, color_index: usize) {
        let sx = x as usize;
        let sy = y as usize;
        let offset = ((sy * 256) + sx) * 2;
        let (top, bottom) = COLORS[color_index];
        buffer[offset] = bottom;
        buffer[offset+1] = top;
    }

    fn draw_tile(&self, mem: &Memory, tile_addr: u16, buffer: &mut [u8], x: u8, y: u8) {
        let mut mut_tile_addr = tile_addr;
        for r in 0..8 {
            let line1 = mem.read8(mut_tile_addr);
            mut_tile_addr += 1;
            let line2 = mem.read8(mut_tile_addr);
            mut_tile_addr += 1;
            for c in 0..8 {
                let mut color_index = (line1 >> (7 - c)) & 1;
                color_index |= if line2 & (0x80 >> c) != 0 { 2 } else { 0 };
                GPU::set_pixel(buffer, x.wrapping_add(c), y.wrapping_add(r), color_index as usize);
            }
        }
    }

    fn draw_sprite(&self, mem: &Memory, src_addr: u16, buffer: &mut [u8], x: u8, y: u8, flags: u8) {
        let mut mut_src_addr = src_addr;
        for r in 0..8 {
            let line1 = mem.read8(mut_src_addr);
            mut_src_addr += 1;
            let line2 = mem.read8(mut_src_addr);
            mut_src_addr += 1;
            for c in 0..8 {
                let mut color_index = (line1 >> (7 - c)) & 1;
                color_index |= if line2 & (0x80 >> c) != 0 { 2 } else { 0 };
                if color_index > 0 {
                    let mut gx = x.wrapping_add(c);
                    let mut gy = y.wrapping_add(r);
                    if (flags & SPRITE_FLIP_H) != 0 { gx = x.wrapping_add(7-c); }
                    if (flags & SPRITE_FLIP_V) != 0 { gy = y.wrapping_add(7-r); }
                    GPU::set_pixel(buffer, gx, gy, color_index as usize);
                }
            }
        }
    }

    fn get_tile_addr(&self, mem: &mut Memory, tile_id: u8) -> u16 {
        if *mem.reg_lcdc() & LCDC_BG_TILE_DATA != 0 {
            (tile_id as u16) * 16 + 0x8000
        } else {
            let tile_sid = (tile_id as i8) as i16 * 16;
            0x9000_u16.wrapping_add(tile_sid as u16)
        }
    }

    pub fn draw_screen(&self, mem: &mut Memory, buffer: &mut [u8]) {
        let mut tile_id: u8;
        let mut tile_addr: u16;
        let mut tile_ptr: u16 = 0x9800;

        if *mem.reg_lcdc() & LCDC_BG_TILE_MAP_SELECT != 0 {
            tile_ptr = 0x9c00;
        }

        // Tiles
        for r in 0..32 {
            for c in 0..32 {
                tile_id = mem.read8(tile_ptr);
                tile_ptr += 1;
                tile_addr = self.get_tile_addr(mem, tile_id);
                let scx = *mem.reg_scx();
                let scy = *mem.reg_scy();
                self.draw_tile(
                    mem,
                    tile_addr,
                    buffer,
                    ((c * 8u16).wrapping_sub(scx as u16) & 0xff).try_into().unwrap(),
                    ((r * 8u16).wrapping_sub(scy as u16) & 0xff).try_into().unwrap(),
                );
            }
        }

        let mut win_ptr: u16 = 0x9800;

        if *mem.reg_lcdc() & LCDC_WINDOW_TILE_MAP_SELECT != 0 {
            win_ptr = 0x9c00;
        }

        // Window
        if *mem.reg_lcdc() & LCDC_WINDOW_ON != 0 {
            for r in 0..32 {
                for c in 0..32 {
                    tile_id = mem.read8(win_ptr);
                    win_ptr += 1;
                    tile_addr = self.get_tile_addr(mem, tile_id);
                    let x = (c * 8u8).wrapping_add(*mem.reg_wx()).wrapping_sub(7);
                    let y = (r * 8u8).wrapping_add(*mem.reg_wy());
                    if tile_id != 0 && x < 167 && y < 144 {
                        self.draw_tile(mem, tile_addr, buffer, x, y);
                    }
                }
            }
        }

        // Sprites
        let mut id: u16;
        let mut flags: u8;
        let mut sprite_addr: u16 = 0xfe00;
        if *mem.reg_lcdc() & LCDC_SHOW_SPRITES != 0 {
            for _r in 0..40 {
                let y = mem.read8(sprite_addr).wrapping_sub(16);
                sprite_addr += 1;
                let x = mem.read8(sprite_addr).wrapping_sub(8);
                sprite_addr += 1;
                id = mem.read8(sprite_addr) as u16;
                sprite_addr += 1;
                flags = mem.read8(sprite_addr);
                sprite_addr += 1;

                if x == 0 && y == 0 {
                    continue;
                }
                self.draw_sprite(mem, id * 16 + 0x8000, buffer, x, y, flags);

                if *mem.reg_lcdc() & LCDC_SPRITE_DOUBLE_HEIGHT != 0 {
                    self.draw_sprite(mem, id * 16 + 0x8000 + 16, buffer, x, y + 8, flags);
                }
            }
        }
    }
}
