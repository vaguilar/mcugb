use crate::memory::Memory;
use crate::memory_types::PPUMode;
use bitmask_enum::bitmask;

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

static SPRITE_PRIORITY: u8 = 1 << 7;
static SPRITE_FLIP_V: u8 = 1 << 6;
static SPRITE_FLIP_H: u8 = 1 << 5;

static COLORS: [(u8, u8); 4] = [(0xe7, 0x9c), (0x97, 0x08), (0x44, 0x31), (0x31, 0x6a)];

pub struct PPU {
    pub clock: u16,
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
    #[inline] fn screen_x(&self) -> i16 { self.x as i16 - 8 }
    #[inline] fn screen_y(&self) -> i16 { self.y as i16 - 16 }

    fn bg_to_object_priority(&self) -> bool { (self.sprite_flags & SPRITE_PRIORITY) != 0 }
    fn flip_vertical(&self) -> bool { (self.sprite_flags & SPRITE_FLIP_V) != 0 }
    fn flip_horizontal(&self) -> bool { (self.sprite_flags & SPRITE_FLIP_H) != 0 }

    fn pixel(&self, mem: &Memory, x: u8, ly: u8, sprite_height: u16) -> Option<u8> {
        let screen_x = self.screen_x();
        let x = x as i16;
        if x < screen_x || x >= screen_x + 8 {
            return None;
        }

        let tile_id = sprite_tile_id(self.tile_id, sprite_height as u8) as u16;
        let tile_addr = (tile_id * 16) + 0x8000;
        let mut sprite_y = (ly as i16 - self.screen_y()) as u16;
        let mut sprite_x = (x - screen_x) as u16;
        if self.flip_horizontal() { sprite_x = 7 - sprite_x; }
        if self.flip_vertical() { sprite_y = sprite_height - 1 - sprite_y; }

        let low = mem.read8(tile_addr + (2 * sprite_y));
        let high = mem.read8(tile_addr + (2 * sprite_y + 1));
        let shift = 7 - sprite_x;
        let pixel = ((low >> shift) & 1) | (((high >> shift) & 1) << 1);
        (pixel != 0).then_some(pixel)
    }
}

fn read_oam_sprite(mem: &Memory, index: usize) -> OAMSprite {
    let start = index * 4;
    let bytes = &mem.reg().sprites[start..start + 4];
    OAMSprite {
        y: bytes[0],
        x: bytes[1],
        tile_id: bytes[2],
        sprite_flags: bytes[3],
    }
}

fn sprite_tile_id(tile_id: u8, sprite_height: u8) -> u8 {
    if sprite_height == 16 { tile_id & 0xfe } else { tile_id }
}

impl PPU {
    pub fn new() -> PPU {
        PPU { clock: 0, sprite_buffer: Default::default() }
    }

    pub fn step(&mut self, mem: &mut Memory, buffer: &mut [u8], cycles: u16) -> (bool, bool) {
        let mut redraw = false;
        let mut vblank = false;
        self.clock += cycles;

        match mem.reg().lcd_stat.ppu_mode() {
            PPUMode::HBlank => {
                if self.clock >= 204 {
                    self.clock -= 204;
                    let lcd_y = mem.reg().lcd_y.wrapping_add(1);
                    Self::set_ly(mem, lcd_y);

                    if mem.reg().lcd_y == 144 {
                        mem.reg_mut().lcd_stat.set_ppu_mode(PPUMode::VBlank);
                        vblank = true;
                        redraw = true;
                    } else {
                        mem.reg_mut().lcd_stat.set_ppu_mode(PPUMode::OAMScan);
                    }
                }
            }
            PPUMode::VBlank => {
                if self.clock >= 456 {
                    self.clock -= 456;
                    let lcd_y = mem.reg().lcd_y.wrapping_add(1);

                    if lcd_y > 153 {
                        Self::set_ly(mem, 0);
                        mem.reg_mut().lcd_stat.set_ppu_mode(PPUMode::OAMScan);
                    } else {
                        Self::set_ly(mem, lcd_y);
                    }
                }
            }
            PPUMode::OAMScan => {
                // OAM read mode
                if self.clock >= 80 {
                    // during this mode, we'll search all sprites ($fe00-$fe9f)
                    // that overlap with this scanline, and store them in our
                    // sprite buffer
                    let ly = mem.reg().lcd_y;
                    let sprite_height = if mem.reg().lcd_control.obj_double_height() { 16 } else { 8 };
                    let mut i = 0;
                    for index in 0..40 {
                        let sprite = read_oam_sprite(mem, index);
                        let sprite_y_end = sprite.screen_y() + sprite_height;
                        let ly = ly as i16;
                        if sprite.x != 0 && ly >= sprite.screen_y() && ly < sprite_y_end {
                            self.sprite_buffer[i] = sprite;
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
                    mem.reg_mut().lcd_stat.set_ppu_mode(PPUMode::Drawing);
                }
            }
            PPUMode::Drawing => {
                // VRAM read mode, pixel transfer, etc
                if self.clock >= 172 {
                    self.clock -= 172;
                    mem.reg_mut().lcd_stat.set_ppu_mode(PPUMode::HBlank);
                    self.draw_scanline(mem, buffer);
                }
            }
        }
        (redraw, vblank)
    }

    fn set_ly(mem: &mut Memory, ly: u8) {
        let lyc = mem.reg().lcd_yc;
        let registers = mem.reg_mut();
        registers.lcd_y = ly;
        registers.lcd_stat.set_lyc_equal(ly == lyc);
    }

    fn draw_scanline(&mut self, mem: &mut Memory, buffer: &mut [u8]) {
        let ly = mem.reg().lcd_y;
        let scx = mem.reg().lcd_scx;
        let scy = mem.reg().lcd_scy;
        let bg_y: u16 = (ly.wrapping_add(scy) / 8).into();
        let bg_py = (ly.wrapping_add(scy) % 8) as u16;

        let bg_tile_map: u16 = if mem.reg().lcd_control.bg_tile_map() {
            0x9c00
        } else {
            0x9800
        };
        let window_tile_map: u16 = if mem.reg().lcd_control.window_tile_map_data_area() {
            0x9c00
        } else {
            0x9800
        };
        let window_start_x = mem.reg().wx as i16 - 7;
        let window_start_y = mem.reg().wy;
        let window_line = ly.wrapping_sub(window_start_y);
        let window_active = mem.reg().lcd_control.window_enable() && ly >= window_start_y;

        for x in 0..160u8 {
            let window_x = x as i16 - window_start_x;
            let use_window = window_active && window_x >= 0;
            let (tile_map, tile_x, tile_y, px, py) = if use_window {
                let window_x = window_x as u16;
                (
                    window_tile_map,
                    window_x / 8,
                    window_line as u16 / 8,
                    (window_x % 8) as u8,
                    window_line as u16 % 8,
                )
            } else {
                let background_x = x.wrapping_add(scx);
                (
                    bg_tile_map,
                    (background_x / 8) as u16,
                    bg_y,
                    background_x % 8,
                    bg_py,
                )
            };
            let tile_id = mem.read8(tile_map + (tile_y * 32 + tile_x));
            let bg_tile_addr = self.get_tile_addr(mem, tile_id);
            let line1 = mem.read8(bg_tile_addr + (2 * py));
            let line2 = mem.read8(bg_tile_addr + (2 * py + 1));
            let shift = 7 - px;
            let bg_pixel = ((line1 >> shift) & 1) | (((line2 >> shift) & 1) << 1);

            let sprite_height = if mem.reg().lcd_control.obj_double_height() { 16 } else { 8 };
            let sprite = if mem.reg().lcd_control.obj_enable() {
                self.sprite_buffer.iter().enumerate().filter_map(|(oam_order, sprite)| {
                    sprite.pixel(mem, x, ly, sprite_height).map(|pixel| {
                        (sprite.x, oam_order, sprite, pixel)
                    })
                }).min_by_key(|(sprite_x, oam_order, _, _)| (*sprite_x, *oam_order))
            } else {
                None
            };
            let (sprite_pixel, bg_to_object_priority) = if let Some((_, _, sprite, pixel)) = sprite {
                (pixel, sprite.bg_to_object_priority())
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

    fn get_tile_addr(&self, mem: &mut Memory, tile_id: u8) -> u16 {
        if mem.reg().lcd_control.bg_and_window_tile_data_area() {
            (tile_id as u16) * 16 + 0x8000
        } else {
            let tile_sid = (tile_id as i8) as i16 * 16;
            0x9000_u16.wrapping_add(tile_sid as u16)
        }
    }

}

#[cfg(test)]
mod tests {
    use super::{read_oam_sprite, sprite_tile_id, OAMSprite, PPU};
    use crate::memory::Memory;
    use crate::memory_types::{LCDControl, PPUMode};

    fn assert_pixel_color(buffer: &[u8], x: usize, y: usize, color: usize) {
        let offset = ((y * 256) + x) * 2;
        let (top, bottom) = super::COLORS[color];
        assert_eq!(&buffer[offset..offset + 2], &[bottom, top]);
    }

    #[test]
    fn oam_scan_reads_sprites_from_register_storage() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        memory.reg_mut().sprites[4..8].copy_from_slice(&[0x20, 0x30, 0x07, 0x80]);

        let sprite = read_oam_sprite(&memory, 1);

        assert_eq!((sprite.y, sprite.x, sprite.tile_id, sprite.sprite_flags),
                   (0x20, 0x30, 0x07, 0x80));
    }

    #[test]
    fn sprite_tile_id_only_aligns_16_pixel_sprites() {
        assert_eq!(sprite_tile_id(0x07, 8), 0x07);
        assert_eq!(sprite_tile_id(0x07, 16), 0x06);
    }

    #[test]
    fn ppu_mode_transitions_are_stored_in_stat() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();

        assert_eq!(memory.reg().lcd_stat.ppu_mode(), PPUMode::VBlank);

        memory.reg_mut().lcd_y = 153;
        ppu.step(&mut memory, &mut buffer, 456);
        assert_eq!(memory.reg().lcd_stat.ppu_mode(), PPUMode::OAMScan);

        ppu.step(&mut memory, &mut buffer, 80);
        assert_eq!(memory.reg().lcd_stat.ppu_mode(), PPUMode::Drawing);

        ppu.step(&mut memory, &mut buffer, 172);
        assert_eq!(memory.reg().lcd_stat.ppu_mode(), PPUMode::HBlank);

        ppu.step(&mut memory, &mut buffer, 204);
        assert_eq!(memory.reg().lcd_stat.ppu_mode(), PPUMode::OAMScan);
    }

    #[test]
    fn lyc_coincidence_tracks_scanline_changes() {
        const LYC_EQUAL: u8 = 1 << 2;

        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();

        memory.reg_mut().lcd_stat.set_ppu_mode(PPUMode::HBlank);
        memory.reg_mut().lcd_y = 40;
        memory.reg_mut().lcd_yc = 41;
        ppu.step(&mut memory, &mut buffer, 204);
        assert_ne!(memory.read8(0xff41) & LYC_EQUAL, 0);

        memory.reg_mut().lcd_stat.set_ppu_mode(PPUMode::HBlank);
        ppu.step(&mut memory, &mut buffer, 204);
        assert_eq!(memory.read8(0xff41) & LYC_EQUAL, 0);

        memory.reg_mut().lcd_stat.set_ppu_mode(PPUMode::VBlank);
        memory.reg_mut().lcd_y = 153;
        memory.reg_mut().lcd_yc = 0;
        ppu.step(&mut memory, &mut buffer, 456);
        assert_eq!(memory.reg().lcd_y, 0);
        assert_ne!(memory.read8(0xff41) & LYC_EQUAL, 0);
    }

    #[test]
    fn background_decodes_known_tile_row() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits(1 << 4);
        memory.write8(0x9800, 0);
        memory.write8(0x8000, 0b1010_0000);
        memory.write8(0x8001, 0b0110_0000);

        ppu.draw_scanline(&mut memory, &mut buffer);

        for (x, color) in [1, 2, 3, 0].iter().copied().enumerate() {
            let (top, bottom) = super::COLORS[color];
            assert_eq!(&buffer[x * 2..x * 2 + 2], &[bottom, top]);
        }
    }

    #[test]
    fn background_scroll_x_wraps_between_tiles() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits(1 << 4);
        memory.reg_mut().lcd_scx = 255;
        memory.write8(0x981f, 1);
        memory.write8(0x9800, 2);
        memory.write8(0x8010, 0b0000_0001);
        memory.write8(0x8011, 0);
        memory.write8(0x8020, 0);
        memory.write8(0x8021, 0b1000_0000);

        ppu.draw_scanline(&mut memory, &mut buffer);

        let (top, bottom) = super::COLORS[1];
        assert_eq!(&buffer[0..2], &[bottom, top]);
        let (top, bottom) = super::COLORS[2];
        assert_eq!(&buffer[2..4], &[bottom, top]);
    }

    #[test]
    fn window_with_wx_below_seven_starts_offscreen() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits(0b0111_0000);
        memory.reg_mut().wx = 0;
        memory.write8(0x9c00, 1);
        memory.write8(0x8010, 0b0000_0001);

        ppu.draw_scanline(&mut memory, &mut buffer);

        assert_pixel_color(&buffer, 0, 0, 1);
        assert_pixel_color(&buffer, 1, 0, 0);
    }

    #[test]
    fn window_at_wx_seven_starts_at_wy() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits(0b0111_0000);
        memory.reg_mut().wx = 7;
        memory.reg_mut().wy = 5;
        memory.write8(0x9c00, 1);
        memory.write8(0x8010, 0b1000_0000);

        memory.reg_mut().lcd_y = 4;
        ppu.draw_scanline(&mut memory, &mut buffer);
        assert_pixel_color(&buffer, 0, 4, 0);

        memory.reg_mut().lcd_y = 5;
        ppu.draw_scanline(&mut memory, &mut buffer);
        assert_pixel_color(&buffer, 0, 5, 1);
    }

    #[test]
    fn window_beyond_visible_screen_does_not_render() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits(0b0111_0000);
        memory.reg_mut().wx = 167;
        memory.write8(0x9800, 0);
        memory.write8(0x9c00, 1);
        memory.write8(0x8001, 0xff);
        memory.write8(0x8010, 0xff);

        ppu.draw_scanline(&mut memory, &mut buffer);

        assert_pixel_color(&buffer, 0, 0, 2);
        assert_pixel_color(&buffer, 159, 0, 2);
    }

    #[test]
    fn transparent_sprite_pixel_reveals_overlapping_sprite() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits(1 << 1);
        memory.write8(0x8010, 0b1000_0000);
        ppu.sprite_buffer[0] = OAMSprite { y: 16, x: 8, tile_id: 0, sprite_flags: 0 };
        ppu.sprite_buffer[1] = OAMSprite { y: 16, x: 8, tile_id: 1, sprite_flags: 0 };

        ppu.draw_scanline(&mut memory, &mut buffer);

        assert_pixel_color(&buffer, 0, 0, 1);
    }

    #[test]
    fn equal_x_uses_earlier_opaque_oam_entry() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits(1 << 1);
        memory.write8(0x8001, 0b1000_0000);
        memory.write8(0x8010, 0b1000_0000);
        ppu.sprite_buffer[0] = OAMSprite { y: 16, x: 8, tile_id: 0, sprite_flags: 0 };
        ppu.sprite_buffer[1] = OAMSprite { y: 16, x: 8, tile_id: 1, sprite_flags: 0 };

        ppu.draw_scanline(&mut memory, &mut buffer);

        assert_pixel_color(&buffer, 0, 0, 2);
    }

    #[test]
    fn smaller_x_sprite_has_priority_over_earlier_oam_entry() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits(1 << 1);
        memory.write8(0x8000, 0b1000_0000);
        memory.write8(0x8011, 0b0100_0000);
        ppu.sprite_buffer[0] = OAMSprite { y: 16, x: 9, tile_id: 0, sprite_flags: 0 };
        ppu.sprite_buffer[1] = OAMSprite { y: 16, x: 8, tile_id: 1, sprite_flags: 0 };

        ppu.draw_scanline(&mut memory, &mut buffer);

        assert_pixel_color(&buffer, 1, 0, 2);
    }

    #[test]
    fn vertically_flipped_8_pixel_sprite_uses_its_last_row_first() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits(1 << 1);
        memory.write8(0x800e, 0b1000_0000);
        ppu.sprite_buffer[0] = OAMSprite {
            y: 16,
            x: 8,
            tile_id: 0,
            sprite_flags: super::SPRITE_FLIP_V,
        };

        ppu.draw_scanline(&mut memory, &mut buffer);

        assert_pixel_color(&buffer, 0, 0, 1);
    }

    #[test]
    fn vertically_flipped_16_pixel_sprite_uses_its_second_tile_first() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_control = LCDControl::from_bits((1 << 2) | (1 << 1));
        memory.write8(0x801e, 0b1000_0000);
        ppu.sprite_buffer[0] = OAMSprite {
            y: 16,
            x: 8,
            tile_id: 1,
            sprite_flags: super::SPRITE_FLIP_V,
        };

        ppu.draw_scanline(&mut memory, &mut buffer);

        assert_pixel_color(&buffer, 0, 0, 1);
    }

    #[test]
    fn sprite_coordinates_allow_partial_offscreen_positions() {
        let sprite = OAMSprite { x: 1, y: 1, ..Default::default() };

        assert_eq!(sprite.screen_x(), -7);
        assert_eq!(sprite.screen_y(), -15);
    }

    #[test]
    fn oam_scan_keeps_partially_visible_left_edge_sprites() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();
        memory.reg_mut().lcd_y = 0;
        memory.reg_mut().sprites[..8].copy_from_slice(&[
            16, 0, 1, 0,
            16, 1, 2, 0,
        ]);
        memory.reg_mut().lcd_stat.set_ppu_mode(PPUMode::OAMScan);

        ppu.step(&mut memory, &mut buffer, 80);

        assert_eq!(ppu.sprite_buffer[0].x, 1);
        assert_eq!(ppu.sprite_buffer[0].tile_id, 2);
    }

    #[test]
    fn vblank_starts_at_scanline_144() {
        let rom = [0; 0x150];
        let mut memory = Memory::with_rom_buffer(&rom);
        let mut buffer = vec![0; 256 * 144 * 2];
        let mut ppu = PPU::new();

        memory.reg_mut().lcd_stat.set_ppu_mode(PPUMode::HBlank);
        memory.reg_mut().lcd_y = 142;
        let (redraw, vblank) = ppu.step(&mut memory, &mut buffer, 204);

        assert_eq!(memory.reg().lcd_y, 143);
        assert!(!redraw);
        assert!(!vblank);

        memory.reg_mut().lcd_stat.set_ppu_mode(PPUMode::HBlank);
        let (redraw, vblank) = ppu.step(&mut memory, &mut buffer, 204);

        assert_eq!(memory.reg().lcd_y, 144);
        assert!(redraw);
        assert!(vblank);
    }
}
