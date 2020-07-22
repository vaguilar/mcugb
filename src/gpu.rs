use crate::memory::Memory;

/*
#define SBUFFER_WIDTH 256;
#define SBUFFER_HEIGHT 256;

#define REG_LCDC	MEM[0xff40]
#define REG_STAT 	MEM[0xff41]
#define REG_SCY		MEM[0xff42]
#define REG_SCX		MEM[0xff43]
#define REG_LY 		MEM[0xff44]
#define REG_LYC		MEM[0xff45]
#define REG_DMA		MEM[0xff46]
#define REG_WY		MEM[0xff4a]
#define REG_WX		MEM[0xff4b]
*/

static LCDC_ON: u8 = 1 << 7;
static LCDC_WINDOW_TILE_MAP_SELECT: u8 = 1 << 6;
static LCDC_WINDOW_ON: u8 = 1 << 5;
static LCDC_BG_TILE_DATA: u8 = 1 << 4;
static LCDC_BG_TILE_MAP_SELECT: u8 = 1 << 3;
static LCDC_SPRITE_DOUBLE_HEIGHT: u8 = 1 << 2;
static LCDC_SHOW_SPRITES: u8 = 1 << 1;
static LCDC_SHOW_BG: u8 = 1 << 0;

/*
#define SPRITE_PRIORITY (1 << 7)
#define SPRITE_FLIP_V (1 << 6)
#define SPRITE_FLIP_H (1 << 5)
*/

pub struct GPU {
    pub mode: u8,
}

impl GPU {
    pub fn step(&self) -> u16 {
        0
    }

    fn draw_tile(&self, tile_addr: u16, buffer: &mut [u8], x: u16, y: u16) {}

    fn draw_sprite(&self, id: u16, buffer: &mut [u8], x: u16, y: u16, flags: u8) {}

    fn get_tile_addr(&self, tile_id: u8) -> u16 {
        0
    }

    pub fn draw_screen(&self, mem: &mut Memory, buffer: &mut [u8]) {
        // for now just draw all the tiles
        let mut tile_id: u8 = 0;
        let mut tile_addr: u16 = 0;
        let mut tile_ptr: u16 = 0x9800;

        if *mem.reg_lcdc() & LCDC_BG_TILE_MAP_SELECT != 0 {
            tile_ptr = 0x9c00;
        }

        for r in 0..32 {
            for c in 0..32 {
                tile_id = mem.read8(tile_ptr);
                tile_addr = self.get_tile_addr(tile_id);
                self.draw_tile(
                    tile_addr,
                    buffer,
                    c * 8 - (*mem.reg_scx() as u16),
                    r * 8 - (*mem.reg_scy() as u16),
                );
                tile_ptr += 1;
            }
        }

        let mut win_ptr: u16 = 0x9800;

        if *mem.reg_lcdc() & LCDC_WINDOW_TILE_MAP_SELECT != 0 {
            win_ptr = 0x9c00;
        }

        if *mem.reg_lcdc() & LCDC_WINDOW_ON != 0 {
            for r in 0..32 {
                for c in 0..32 {
                    tile_id = mem.read8(win_ptr);
                    win_ptr += 1;
                    tile_addr = self.get_tile_addr(tile_id);
                    let x = c * 8 + (*mem.reg_wx() as u16) - 7;
                    let y = r * 8 + (*mem.reg_wy() as u16);
                    if tile_id != 0 && x < 167 && y < 144 {
                        self.draw_tile(tile_addr, buffer, x, y);
                    }
                }
            }
        }

        let mut id: u16;
        let mut flags: u8;
        let mut sprite_addr: u16 = 0xfe00;

        if *mem.reg_lcdc() & LCDC_SHOW_SPRITES != 0 {
            for r in 0..40 {
                let y = (mem.read8(sprite_addr) - 16) as u16;
                sprite_addr += 1;
                let x = (mem.read8(sprite_addr) - 8) as u16;
                sprite_addr += 1;
                id = mem.read8(sprite_addr) as u16;
                sprite_addr += 1;
                flags = mem.read8(sprite_addr);
                sprite_addr += 1;

                if x == 0 && y == 0 {
                    continue;
                }
                self.draw_sprite(id * 16 + 0x8000, buffer, x, y, flags);

                if *mem.reg_lcdc() & LCDC_SPRITE_DOUBLE_HEIGHT != 0 {
                    self.draw_sprite(id * 16 + 0x8000 + 16, buffer, x, y + 8, flags);
                }
            }
        }
    }
}
