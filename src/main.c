#include <stdio.h>
#include <stdint.h>

#include <SDL.h>

#include "tests.h"
#include "cpu.h"
#include "mem.h"
#include "gpu.h"

SDL_Window* Window = NULL;
SDL_Renderer* Renderer = NULL;
SDL_Surface* PrimarySurface = NULL;
SDL_Texture* Texture = NULL;
SDL_Rect SrcRect;
SDL_Rect DestRect;

int init_win() {
	if(SDL_Init(SDL_INIT_VIDEO) < 0) {
		printf("Unable to Init SDL: %s", SDL_GetError());
		return 0;
	}

	if (SDL_CreateWindowAndRenderer(160, 144, SDL_WINDOW_RESIZABLE, &Window, &Renderer)) {
		printf("Unable to create SDL Renderer: %s\n", SDL_GetError());
		return 0;
	}

	/* set 2x */
	SDL_SetWindowSize(Window, 160 * 2, 144 * 2);

	SDL_SetHint(SDL_HINT_RENDER_SCALE_QUALITY, "nearest");
	SDL_RenderSetLogicalSize(Renderer, 160, 144);

	SDL_SetWindowTitle(Window, "mcugb");
	Texture = SDL_CreateTexture(Renderer, SDL_PIXELFORMAT_ARGB4444, SDL_TEXTUREACCESS_STREAMING, 256, 256);

	//PrimarySurface = SDL_GetWindowSurface(Window);
	SDL_SetRenderDrawColor(Renderer, 0x30, 0x30, 0x30, 0xFF);
	SDL_RenderClear(Renderer);
	SDL_RenderPresent(Renderer);

	return 1;
}

int main(int argc, char **argv) {
	FILE *fp;
	uint32_t i, j, cycles, total_cycles;
	uint16_t buffer[256 * 256] = {0};
	uint8_t redraw = 0;
	SDL_Event Event;

	run_tests();

	if (argc < 2) {
		printf("usage: ./mcugb romfile");
		exit(0);
	}

	fp = fopen(argv[1], "r");
	fread(MEM, 32768, 1, fp);
	fclose(fp);

	cpu_reset();
	REG_PC = 0x0100;

	memset(buffer, 0, 256 * 256);

	if(init_win() == 0) {
		printf("Unable to init window");
		return 0;
	}

	while (1) {
		//if (REG_PC == 0x039f) { printf("BREAK\n"); break; }
		cycles = cpu_step();
		redraw = gpu_step(cycles, buffer);
		//cpu_debug();

		SDL_PollEvent(&Event);
		if (Event.type == SDL_QUIT) break;
		if (Event.type == SDL_KEYDOWN) {
			if (Event.key.type == SDL_KEYDOWN) {
				switch (Event.key.keysym.sym) {
				case SDLK_DOWN:
					cpu_set_joypad(1, 3);
					break;
				case SDLK_UP:
					cpu_set_joypad(1, 2);
					break;
				case SDLK_LEFT:
					cpu_set_joypad(1, 1);
					break;
				case SDLK_RIGHT:
					cpu_set_joypad(1, 0);
					break;
				}
			} else {
				switch (Event.key.keysym.sym) {
				case SDLK_DOWN:
					cpu_unset_joypad(1, 3);
					break;
				case SDLK_UP:
					cpu_unset_joypad(1, 2);
					break;
				case SDLK_LEFT:
					cpu_unset_joypad(1, 1);
					break;
				case SDLK_RIGHT:
					cpu_unset_joypad(1, 0);
					break;
				}
			}
		}

		/* draw buffer */
		if (redraw == 1) {
			gpu_draw_screen(buffer);
			SDL_UpdateTexture(Texture, NULL, buffer, 256 * sizeof(uint16_t));
			SDL_RenderClear(Renderer);

			SrcRect.x = 0;
			SrcRect.y = 0;
			SrcRect.w = 256;
			SrcRect.h = 256;
			DestRect.x = 0;
			DestRect.y = 0;
			DestRect.w = 256;
			DestRect.h = 256;
			SDL_RenderCopy(Renderer, Texture, &SrcRect, &DestRect);

			SDL_RenderPresent(Renderer);
		}

		total_cycles += cycles;
	}

	cpu_debug();
	mem_debug(0x9000, 128);
	mem_debug(0x9800, 128);
	printf("\n");
	mem_debug(0xfe00, 128);

	SDL_DestroyTexture(Texture);
	SDL_DestroyRenderer(Renderer);
	SDL_DestroyWindow(Window);
	SDL_Quit();
}
