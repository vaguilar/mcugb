#include <stdio.h>
#include <stdint.h>
#include <pthread.h>

#include <SDL.h>

#include "tests.h"
#include "cpu.h"
#include "mem.h"
#include "gpu.h"
#include "debugger.h"

SDL_Window* Window = NULL;
SDL_Renderer* Renderer = NULL;
SDL_Surface* PrimarySurface = NULL;
SDL_Texture* Texture = NULL;
SDL_Rect SrcRect;
SDL_Rect DestRect;

pthread_t debugger_thread;
pthread_mutex_t mutex;
volatile uint8_t RUNNING = 0;

uint8_t init_win() {
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

uint8_t handle_event(SDL_Event *Event) {
	if (Event->type == SDL_QUIT) {
		return 1;

	} else if (Event->type == SDL_KEYDOWN) {
		switch (Event->key.keysym.sym) {
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
		case SDLK_v:
			cpu_set_joypad(0, 3);
			break;
		case SDLK_c:
			cpu_set_joypad(0, 2);
			break;
		case SDLK_z:
			cpu_set_joypad(0, 1);
			break;
		case SDLK_x:
			cpu_set_joypad(0, 0);
			break;
		}

	} else if (Event->type == SDL_KEYUP) {
		switch (Event->key.keysym.sym) {
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
		case SDLK_v:
			cpu_unset_joypad(0, 3);
			break;
		case SDLK_c:
			cpu_unset_joypad(0, 2);
			break;
		case SDLK_z:
			cpu_unset_joypad(0, 1);
			break;
		case SDLK_x:
			cpu_unset_joypad(0, 0);
			break;
		case SDLK_ESCAPE:
			/* pause game */
			debugger_set_state(0);
		}
	}
	return 0;
}

uint8_t main(int argc, char **argv) {
	FILE *fp;
	uint32_t i, j, cycles;
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
		return 1;
	}

	if(pthread_create(&debugger_thread, 0, debugger_main, &RUNNING)) {
		printf("Unable to start debugger thread");
		return 1;
	}

	while (1) {
		if (RUNNING) {
			if (debugger_in_breakpoints(REG_PC)) {
				RUNNING = 0;
				printf("Triggered breakpoint at $%04hx.\n", REG_PC);
				debugger_set_state(0);
			} else {
				cycles = cpu_step();
				redraw = gpu_step(cycles, buffer);
				//cpu_debug();
			}
		}

		if (SDL_PollEvent(&Event)) {
			if (handle_event(&Event)) break;
		}

		/* draw buffer */
		if (redraw == 1 || RUNNING == 0) {
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
			SDL_Delay(1);
		}
	}

	cpu_debug();
	mem_debug(0x9000, 128);
	mem_debug(0x9800, 128);
	printf("\n");
	mem_debug(0xff40, 16);
	printf("\n");
	mem_debug(0xfe00, 128);
	//printf("STACK:\n");
	//mem_debug(REG_SP, 0xffff - REG_SP > 64 ? 64 : 0xffff - REG_SP);
	mem_debug(0xc0a4, 16);
	mem_debug(0x98a0, 16);
	printf("LCDC: $%04x\n", REG_LCDC);
	printf("STAT: $%04x\n", REG_STAT);
	printf("REG IE: $%04x\n", REG_INTERRUPT_ENABLE);
	printf("REG IF: $%04x\n", REG_INTERRUPT_FLAG);

	SDL_DestroyTexture(Texture);
	SDL_DestroyRenderer(Renderer);
	SDL_DestroyWindow(Window);
	SDL_Quit();

	return 0;
}
