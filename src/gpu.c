#include <stdint.h>

#include "gpu.h"
#include "mem.h"

#define REG_STAT 	MEM[0xff41]
#define REG_LY 		MEM[0xff44]

const uint16_t COLORS[4] = {0x0eee, 0x0999, 0x0444, 0x0000};
uint16_t clock = 0;
uint8_t mode = 0;

/* stubbed with code from here while I figure this out
 * http://imrannazar.com/GameBoy-Emulation-in-JavaScript:-GPU-Timings
 */
void gpu_step(uint16_t cycles) {
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
		//GPU.renderscan();
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
			//GPU._canvas.putImageData(GPU._scrn, 0, 0);
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
}

void gpu_set_pixel(uint16_t *pixels, uint32_t x, uint16_t y, uint16_t color) {
	pixels[(y * 256) + x] = color;
}

void gpu_draw_tile(uint16_t src_addr, uint16_t *dst_buffer, uint32_t x, uint32_t y) {
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
