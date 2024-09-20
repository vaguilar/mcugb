use crate::memory::Memory;
use std::convert::TryInto;
use bitmask_enum::bitmask;

#[bitmask(u8)]
enum LCDControl {
    ShowBackground = 1,
    ShowSprite,
    SpriteDoubleHeight,
    BackgroundTileMapSelect,
    BackgroundTileData,
    WindowOn,
    WindowTileMapSelect,
}

#[bitmask(u8)]
enum SpriteFlags {
    // _ = 1 << 0,
    // _ = 1 << 1,
    // _ = 1 << 2,
    VRAMBank = 1 << 3,
    DMGPalette = 1 << 4,
    FlipHorizontal = 1 << 5,
    FlipVertical = 1 << 6,
    BGPriority = 1 << 7, // if set, bg has priority over this sprite for bg values 1-3
}

// static LCDC_ON: u8 = 1 << 7;
static LCDC_WINDOW_TILE_MAP_SELECT: u8 = 1 << 6;
static LCDC_WINDOW_ON: u8 = 1 << 5;
static LCDC_BG_TILE_DATA: u8 = 1 << 4;
static LCDC_BG_TILE_MAP_SELECT: u8 = 1 << 3;
static LCDC_SPRITE_DOUBLE_HEIGHT: u8 = 1 << 2;
static LCDC_SHOW_SPRITES: u8 = 1 << 1;
// static LCDC_SHOW_BG: u8 = 1 << 0;


static SPRITE_PRIORITY: u8 = 1 << 7;
static SPRITE_FLIP_V: u8 = 1 << 6;
static SPRITE_FLIP_H: u8 = 1 << 5;

static COLORS: [(u8, u8); 4] = [(0xe7, 0x9c), (0x97, 0x08), (0x44, 0x31), (0x31, 0x6a)];

#[repr(u8)]
enum PPUMode {
    HBlank  = 0,
    VBlank  = 1,
    OAMScan = 2,
    Drawing = 3, // VRAM read mode?
}

pub struct PPU {
    pub clock: u16,
    mode: PPUMode,
    sprite_buffer: [OAMSprite; 10],
}

#[derive(Debug, Clone, Default)]
struct OAMSprite {
    y: u8,
    x: u8,
    tile_id: u8,
    sprite_flags: u8,
}

impl OAMSprite {
    #[inline]
    fn adjusted_x(&self) -> u8 { self.x.wrapping_sub(8) }
    #[inline]
    fn adjusted_y(&self) -> u8 { self.y.wrapping_sub(16) }

    fn bg_to_object_priority(&self) -> bool { (self.sprite_flags & SPRITE_PRIORITY) != 0 }
    fn flip_vertical(&self) -> bool { (self.sprite_flags & SPRITE_FLIP_V) != 0 }
    fn flip_horizontal(&self) -> bool { (self.sprite_flags & SPRITE_FLIP_H) != 0 }
}

impl PPU {
    pub fn new() -> PPU {
        PPU { clock: 0, mode: PPUMode::VBlank, sprite_buffer: Default::default() }
    }

    pub fn step(&mut self, mem: &mut Memory, buffer: &mut [u8], cycles: u16) -> (bool, bool) {
        let mut redraw = false;
        let mut vblank = false;
        self.clock += cycles;

        match self.mode {
            PPUMode::HBlank => {
                if self.clock >= 204 {
                    self.clock -= 204;
                    mem.reg.lcd_y = mem.reg.lcd_y.wrapping_add(1);

                    if mem.reg.lcd_y == 143 {
                        self.mode = PPUMode::VBlank;
                        mem.reg.lcd_stat = (mem.reg.lcd_stat & 0xfc) | (PPUMode::VBlank as u8);
                        vblank = true;
                        redraw = true;
                    } else {
                        self.mode = PPUMode::OAMScan;
                        mem.reg.lcd_stat = (mem.reg.lcd_stat & 0xfc) | (PPUMode::OAMScan as u8);
                    }
                }
            }
            PPUMode::VBlank => {
                if self.clock >= 456 {
                    self.clock -= 456;
                    mem.reg.lcd_y = mem.reg.lcd_y.wrapping_add(1);

                    if mem.reg.lcd_y > 153 {
                        mem.reg.lcd_y = 0;
                        self.mode = PPUMode::OAMScan;
                        mem.reg.lcd_stat = (mem.reg.lcd_stat & 0xfc) | (PPUMode::OAMScan as u8);
                    }
                }
            }
            PPUMode::OAMScan => {
                // OAM read mode
                if self.clock >= 80 {
                    // during this mode, we'll search all sprites ($fe00-$fe9f)
                    // that overlap with this scanline, and store them in our
                    // sprite buffer
                    let sprites = unsafe {
                        std::mem::transmute::<&[u8], &[OAMSprite]>(&mem.data[0xfe00..0xfe9f])
                    };
                    let ly = mem.reg.lcd_y;
                    let sprite_height = if mem.reg.lcd_control & LCDC_SPRITE_DOUBLE_HEIGHT != 0 { 16 } else { 8 };
                    let mut i = 0;
                    for sprite in sprites {
                        if sprite.adjusted_x() > 0 && ly >= sprite.adjusted_y() && ly < sprite.adjusted_y() + sprite_height {
                            self.sprite_buffer[i] = sprite.clone();
                            i += 1;
                            if i >= 10 { break }
                        }
                    }
                    while i < 10 { self.sprite_buffer[i] = Default::default(); i += 1; }
                    // self.sprite_buffer = sprites.iter()
                    //     .filter(|sprite| { sprite.adjusted_x() > 0 && ly >= sprite.adjusted_y() && ly < sprite.adjusted_y() + sprite_height })
                    //     .take(10)
                    //     .cloned()
                    //     .collect::<Vec<OAMSprite>>();

                    self.clock -= 80;
                    self.mode = PPUMode::Drawing;
                    mem.reg.lcd_stat = (mem.reg.lcd_stat & 0xfc) | (PPUMode::Drawing as u8);
                }
            }
            PPUMode::Drawing => {
                // VRAM read mode, pixel transfer, etc
                if self.clock >= 172 {
                    self.clock -= 172;
                    self.mode = PPUMode::HBlank;
                    mem.reg.lcd_stat = (mem.reg.lcd_stat & 0xfc) | (PPUMode::HBlank as u8);
                    self.draw_scanline(mem, buffer);
                }
            }
        }
        (redraw, vblank)
    }

    fn draw_scanline(&mut self, mem: &mut Memory, buffer: &mut [u8]) {
        let ly = mem.reg.lcd_y;
        let scx = mem.reg.lcd_scx;
        let scy = mem.reg.lcd_scy;
        let bg_y: u16 = (ly.wrapping_add(scy) / 8).into();
        let py = (ly.wrapping_add(scy) % 8) as u16;

        let tile_ptr: u16 = if mem.reg.lcd_control & LCDC_BG_TILE_MAP_SELECT != 0 {
            0x9c00
        } else {
            0x9800
        };

        for x in 0..160u8 {
            let bg_x: u16 = (x.wrapping_add(scx) / 8).into();
            let px = x.wrapping_add(scx) % 8;
            // dbg!(bg_x, bg_y);
            let tile_id = mem.read8(tile_ptr + (bg_y * 32 + bg_x));
            let bg_tile_addr = self.get_tile_addr(mem, tile_id);
            let line1 = mem.read8(bg_tile_addr + (2 * py));
            let line2 = mem.read8(bg_tile_addr + (2 * py + 1)).rotate_left(1);
            let bg_pixel = (line1.rotate_left(px as u32) & 1) | (line2.rotate_left(px as u32 + 1) & 2);

            let mut sprite = None;
            if mem.reg.lcd_control & LCDC_SHOW_SPRITES != 0 {
                sprite = self.sprite_buffer.iter()
                    .filter(|sprite| sprite.x <= x + 8 && x + 8 < sprite.x + 8)
                    .next();
            }
            let (sprite_pixel, bg_to_object_priority)  = if let Some(sprite) = sprite {
                let sprite_height = if mem.reg.lcd_control & LCDC_SPRITE_DOUBLE_HEIGHT != 0 { 16 } else { 8 };
                let tile_id = (sprite.tile_id as u16) & 0xff;
                let sprite_tile_addr = (tile_id * 16) + 0x8000;
                dbg!(sprite, ly, x);
                let mut py: u16 = (ly - sprite.adjusted_y()).into();
                let mut px: u16 = (x - sprite.adjusted_x()).into();
                dbg!(px, py);
                if sprite.flip_horizontal() { px = 7 - px; }
                if sprite.flip_vertical() { py = sprite_height - py; }
                let line1 = mem.read8(sprite_tile_addr + (2 * py));
                let line2 = mem.read8(sprite_tile_addr + (2 * py + 1));
                // let sprite_pixel = (line1.rotate_left(px as u32) & 1) | (line2.rotate_left(px as u32 + 1) & 2);
                let mut sprite_pixel = (line1 >> (7 - px)) & 1;
                sprite_pixel |= if line2 & (0x80 >> px) != 0 { 2 } else { 0 };
                (sprite_pixel, sprite.bg_to_object_priority())
            } else {
                (0, false)
            };

            let color = match (bg_pixel, sprite_pixel, bg_to_object_priority) {
                (_, 0, _) => bg_pixel,
                (1.., _, true) => bg_pixel,
                _ => sprite_pixel,
            };

            PPU::set_pixel(buffer, x, ly, color as usize);
        }
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
                PPU::set_pixel(buffer, x.wrapping_add(c), y.wrapping_add(r), color_index as usize);
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
                    PPU::set_pixel(buffer, gx, gy, color_index as usize);
                }
            }
        }
    }

    fn get_tile_addr(&self, mem: &mut Memory, tile_id: u8) -> u16 {
        if mem.reg.lcd_control & LCDC_BG_TILE_DATA != 0 {
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

        if mem.reg.lcd_control & LCDC_BG_TILE_MAP_SELECT != 0 {
            tile_ptr = 0x9c00;
        }

        // Tiles
        for r in 0..32 {
            for c in 0..32 {
                tile_id = mem.read8(tile_ptr);
                tile_ptr += 1;
                tile_addr = self.get_tile_addr(mem, tile_id);
                let scx = mem.reg.lcd_scx;
                let scy = mem.reg.lcd_scx;
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

        if mem.reg.lcd_control & LCDC_WINDOW_TILE_MAP_SELECT != 0 {
            win_ptr = 0x9c00;
        }

        // Window
        if mem.reg.lcd_control & LCDC_WINDOW_ON != 0 {
            for r in 0..32 {
                for c in 0..32 {
                    tile_id = mem.read8(win_ptr);
                    win_ptr += 1;
                    tile_addr = self.get_tile_addr(mem, tile_id);
                    let x = (c * 8u8).wrapping_add(mem.reg.wx).wrapping_sub(7);
                    let y = (r * 8u8).wrapping_add(mem.reg.wy);
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
        if mem.reg.lcd_control & LCDC_SHOW_SPRITES != 0 {
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

                if mem.reg.lcd_control & LCDC_SPRITE_DOUBLE_HEIGHT != 0 {
                    self.draw_sprite(mem, id * 16 + 0x8000 + 16, buffer, x, y + 8, flags);
                }
            }
        }
    }
}
