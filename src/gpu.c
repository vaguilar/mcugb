#include <stdint.h>

#include "gpu.h"
#include "mem.h"

const uint16_t COLORS[4] = {0x0eef, 0x099a, 0x0445, 0x0001};
uint32_t clock = 0;
uint8_t mode = 0;

/* stubbed with code from here while I figure this out
 * http://imrannazar.com/GameBoy-Emulation-in-JavaScript:-GPU-Timings
 */
uint8_t gpu_step(uint16_t cycles, uint16_t *buffer) {
	uint8_t redraw = 0;
	clock += cycles;

	switch(mode) {
	// OAM read mode, scanline active
	case 2:
	if(clock >= 80) {
		// Enter scanline mode 3
		clock = 0;
		mode = 3;
		REG_STAT = (REG_STAT & 0xfc) | mode;
	}
	break;

	// VRAM read mode, scanline active
	// Treat end of mode 3 as end of scanline
	case 3:
	if(clock >= 172) {
		// Enter hblank
		clock = 0;
		mode = 0;
		REG_STAT = (REG_STAT & 0xfc) | mode;

		// Write a scanline to the framebuffer
		gpu_draw_scanline();
	}
	break;

	// Hblank
	// After the last hblank, push the screen data to canvas
	case 0:
	if(clock >= 204) {
		clock = 0;
		REG_LY++;

		if(REG_LY == 143) {
			// Enter vblank
			mode = 1;
			REG_STAT = (REG_STAT & 0xfc) | mode;
			//gpu_draw_screen(buffer);
			redraw = 1;
		} else {
			mode = 2;
			REG_STAT = (REG_STAT & 0xfc) | mode;
		}
	}
	break;

	// Vblank (10 lines)
	case 1:
	if(clock >= 456) {
		clock = 0;
		REG_LY++;

		if(REG_LY > 153) {
			// Restart scanning modes
			REG_LY = 0;
			mode = 2;
			REG_STAT = (REG_STAT & 0xfc) | mode;
		}
	}
	break;
	}

	return redraw;
}

void gpu_set_pixel(uint16_t *pixels, uint8_t x, uint8_t y, uint16_t color) {
	uint16_t sx = x & 0xff, sy = y & 0xff;
	pixels[(sy * 256) + sx] = color;
	//pixels[(y << 8) | x] = color;
}

void gpu_draw_tile(uint16_t src_addr, uint16_t *dst_buffer, uint16_t x, uint16_t y) {
	uint32_t r, c, color_index, line1, line2;
	for (r = 0; r < 8; r++) {
		line1 = mem_read8(src_addr++);
		line2 = mem_read8(src_addr++);
		for (c = 0; c < 8; c++) {
			color_index  = (line1 >> (7 - c)) & 1;
			color_index |= line2 & (0x80 >> c) ? 2 : 0;
			gpu_set_pixel(dst_buffer, x+c, y+r, COLORS[color_index]);
		}
	}
}

void gpu_draw_sprite(uint16_t src_addr, uint16_t *dst_buffer, uint16_t x, uint16_t y) {
	uint32_t r, c, color_index, line1, line2;
	for (r = 0; r < 8; r++) {
		line1 = mem_read8(src_addr++);
		line2 = mem_read8(src_addr++);
		for (c = 0; c < 8; c++) {
			color_index  = (line1 >> (7 - c)) & 1;
			color_index |= line2 & (0x80 >> c) ? 2 : 0;
			if (color_index > 0) {
				gpu_set_pixel(dst_buffer, x+c, y+r, COLORS[color_index]);
			}
		}
	}
}

void gpu_draw_scanline() {
	/* TODO */
}

uint16_t get_tile_addr(uint8_t id) {
	int8_t sid = (int8_t) id;
	if (REG_LCDC & LCDC_BG_TILE_DATA) {
		return (id * 16) + 0x8000;
	} else {
		return (sid * 16) + 0x9000;
	}
}

void gpu_draw_screen(uint16_t *buffer) {
	/* for now just draw all the tiles */
	uint16_t r, c, tile_id, tile_addr, tile_ptr = 0x9800;

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

	uint8_t x, y, id, flags;
	uint16_t sprite_addr = 0xfe00;
	if (REG_LCDC & LCDC_SHOW_SPRITES) {
		for (r = 0; r < 40; r++) {
			x = mem_read8(sprite_addr++);
			y = mem_read8(sprite_addr++);
			id = mem_read8(sprite_addr++);
			flags = mem_read8(sprite_addr++);

			if (x == 0 && y == 0) continue;
			gpu_draw_sprite(id * 16 + 0x8000, buffer, x - REG_SCX, y - REG_SCY);

			if (REG_LCDC & LCDC_SPRITE_DOUBLE_HEIGHT) {
				gpu_draw_sprite(id * 16 + 0x8000 + 16, buffer, x - REG_SCX, y + 8 - REG_SCY);
			}
		}
	}
}
