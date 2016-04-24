#pragma once

#define SBUFFER_WIDTH 256;
#define SBUFFER_HEIGHT 256;

#define REG_LCDC	MEM[0xff40]
#define REG_STAT 	MEM[0xff41]
#define REG_SCY		MEM[0xff42]
#define REG_SCX		MEM[0xff43]
#define REG_LY 		MEM[0xff44]
#define REG_LYC		MEM[0xff45]
#define REG_DMA		MEM[0xff46]

#define LCDC_BG_TILE_DATA (1 << 4)
#define LCDC_BG_TILE_MAP_SELECT (1 << 3)
#define LCDC_SPRITE_DOUBLE_HEIGHT (1 << 2)
#define LCDC_SHOW_SPRITES (1 << 1)
#define LCDC_SHOW_BG (1 << 0)

extern const uint16_t COLORS[4];

uint8_t gpu_step(uint16_t cycles, uint16_t *buffer);
void gpu_set_pixel(uint16_t *pixels, uint8_t x, uint8_t y, uint16_t color);
void gpu_draw_tile(uint16_t src_addr, uint16_t *dst_buffer, uint16_t x, uint16_t y);
void gpu_draw_sprite(uint16_t src_addr, uint16_t *dst_buffer, uint16_t x, uint16_t y);
void gpu_draw_scanline();
void gpu_draw_screen();
