#pragma once

#define SCREEN_WIDTH 256;
#define SCREEN_HEIGHT 256;

extern const uint16_t COLORS[4];

void gpu_set_pixel(uint16_t *pixels, uint32_t x, uint16_t y, uint16_t color);
void gpu_step(uint16_t cycles);
void gpu_draw_tile(uint16_t src_addr, uint16_t *dst_buffer, uint32_t x, uint32_t y);
