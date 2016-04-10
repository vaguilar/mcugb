#pragma once

#define SBUFFER_WIDTH 256;
#define SBUFFER_HEIGHT 256;

extern const uint16_t COLORS[4];

uint8_t gpu_step(uint16_t cycles, uint16_t *buffer);
void gpu_set_pixel(uint16_t *pixels, uint16_t x, uint16_t y, uint16_t color);
void gpu_draw_tile(uint16_t src_addr, uint16_t *dst_buffer, uint16_t x, uint16_t y);
void gpu_draw_scanline();
void gpu_draw_screen();
