pub struct GPU {
    mode :u8,
}

impl GPU {
    pub fn step(&self) -> u16 {
        0
    }

    pub fn draw_screen(buffer: &mut[u8]) {
        /* for now just draw all the tiles */
        let r: u16 = 0;
        let c: u16 = 0;
        let tile_id: u16 = 0;
        let tile_addr: u16 = 0;
        let tile_ptr: u16 = 0x9800;

        if (REG_LCDC & LCDC_BG_TILE_MAP_SELECT) {
            tile_ptr = 0x9c00;
        }

        for (r = 0; r < 32; r++) {
            for (c = 0; c < 32; c++) {
                tile_id = mem_read8(tile_ptr++);
                tile_addr = get_tile_addr(tile_id);
                gpu_draw_tile(tile_addr, buffer, c * 8 - REG_SCX, r * 8 - REG_SCY);
            }
        }

        uint16_t x, y, win_ptr = 0x9800;

        if (REG_LCDC & LCDC_WINDOW_TILE_MAP_SELECT) {
            win_ptr = 0x9c00;
        }

        if (REG_LCDC & LCDC_WINDOW_ON) {
            for (r = 0; r < 32; r++) {
                for (c = 0; c < 32; c++) {
                    tile_id = mem_read8(win_ptr++);
                    tile_addr = get_tile_addr(tile_id);
                    x = c * 8 + REG_WX - 7;
                    y = r * 8 + REG_WY;
                    if (tile_id && x < 167 && y < 144) {
                        gpu_draw_tile(tile_addr, buffer, x, y);
                    }
                }
            }
        }

        uint8_t id, flags;
        uint16_t sprite_addr = 0xfe00;
        if (REG_LCDC & LCDC_SHOW_SPRITES) {
            for (r = 0; r < 40; r++) {
                y = mem_read8(sprite_addr++) - 16;
                x = mem_read8(sprite_addr++) - 8;
                id = mem_read8(sprite_addr++);
                flags = mem_read8(sprite_addr++);

                if (x == 0 && y == 0) continue;
                gpu_draw_sprite(id * 16 + 0x8000, buffer, x, y, flags);

                if (REG_LCDC & LCDC_SPRITE_DOUBLE_HEIGHT) {
                    gpu_draw_sprite(id * 16 + 0x8000 + 16, buffer, x, y + 8, flags);
                }
            }
        }
    }
}