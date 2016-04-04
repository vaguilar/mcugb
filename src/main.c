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

int init_win() {
	if(SDL_Init(SDL_INIT_VIDEO) < 0) {
		printf("Unable to Init SDL: %s", SDL_GetError());
		return 0;
	}

	if (SDL_CreateWindowAndRenderer(256, 256, SDL_WINDOW_RESIZABLE, &Window, &Renderer)) {
		printf("Unable to create SDL Renderer: %s\n", SDL_GetError());
		return 0;
	}

	SDL_SetHint(SDL_HINT_RENDER_SCALE_QUALITY, "linear");  // make the scaled rendering look smoother.
	SDL_RenderSetLogicalSize(Renderer, 256, 256);

	SDL_SetWindowTitle(Window, "mcugb");
	Texture = SDL_CreateTexture(Renderer, SDL_PIXELFORMAT_ARGB4444, SDL_TEXTUREACCESS_STREAMING, 256,256);

	//PrimarySurface = SDL_GetWindowSurface(Window);
	SDL_SetRenderDrawColor(Renderer, 0x30, 0x30, 0x30, 0xFF);
	SDL_RenderClear(Renderer);
	SDL_RenderPresent(Renderer);
	return 1;
}

int main(int argc, char **argv) {
	FILE *fp;
	uint32_t i, j, cycles, total_cycles;

	run_tests();

	fp = fopen("SPACE.GB", "r");
	fread(MEM, 32768-1, 1, fp);
	fclose(fp);

	cpu_reset();
	REG_PC = 0x0100;

	for (i = 0; i < 100000; i++) {
		//if (REG_PC == 0x039f) { printf("BREAK\n"); break; }
		cycles = cpu_step();
		gpu_step(cycles);
		cpu_debug();

		total_cycles += cycles;
	}

	mem_debug(0x9000, 128);
	mem_debug(0x9800, 128);

	uint16_t buffer[256 * 256] = {0};
	memset(buffer, 0, 256 * 256);

	if(init_win() == 0) {
		printf("Unable to init window");
		return 0;
	}

	/* draw tiles at 0x9000 to buffer for now */
	for (j = 0; j < 8; j++) {
		for (i = 0; i < 16; i++) {
			int offset = (i * 16) + (j * 256);
			gpu_draw_tile(0x9000 + offset, buffer, i * 8, j * 8);
		}
	}

	/* write buffer */
	SDL_UpdateTexture(Texture, NULL, buffer, 256 * sizeof(uint16_t));

	SDL_Event Event;
	while(1) {
		SDL_PollEvent(&Event);
		if(Event.type == SDL_QUIT) break;
		//Loop();
		//Render();
		//SDL_SetRenderDrawColor(Renderer, 0x00, 0x00, 0x00, 0x00);
		SDL_RenderClear(Renderer);
		SDL_RenderCopy(Renderer, Texture, NULL, NULL);
		SDL_RenderPresent(Renderer);
		SDL_Delay(1); // Breath
	}

	SDL_DestroyTexture(Texture);
	SDL_DestroyRenderer(Renderer);
	SDL_DestroyWindow(Window);
	SDL_Quit();
}
