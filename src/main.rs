extern crate sdl2;

mod cpu;
mod memory;
mod gb;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::env;
use std::thread::sleep;
use std::time::Duration;

static SCALE_FACTOR: u8 = 2;

/*
pthread_t debugger_thread;
pthread_mutex_t mutex;
volatile uint8_t RUNNING = 0;
volatile uint8_t STEP = 0;

uint8_t init_win() {
    if(SDL_Init(SDL_INIT_VIDEO) < 0) {
        printf("Unable to Init SDL: %s", SDL_GetError());
        return 0;
    }

    if (SDL_CreateWindowAndRenderer(160, 144, SDL_WINDOW_RESIZABLE, &Window, &Renderer)) {
        printf("Unable to create SDL Renderer: %s\n", SDL_GetError());
        return 0;
    }

    /* set scale */
    SDL_SetWindowSize(Window, 160 * SCALE_FACTOR, 144 * SCALE_FACTOR);

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
*/

fn handle_event(event: &Event, gb: &mut gb::GB) -> bool {
    match event {
        Event::Quit { .. } | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => return true,

        // KeyDown
        Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
            gb.set_joypad(1, 3);
        }
        Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
            gb.set_joypad(1, 2);
        }
        Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
            gb.set_joypad(1, 1);
        }
        Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
            gb.set_joypad(1, 0);
        }
        Event::KeyDown { keycode: Some(Keycode::V), .. } => {
            gb.set_joypad(0, 3);
        }
        Event::KeyDown { keycode: Some(Keycode::C), .. } => {
            gb.set_joypad(0, 2);
        }
        Event::KeyDown { keycode: Some(Keycode::Z), .. } => {
            gb.set_joypad(0, 1);
        }
        Event::KeyDown { keycode: Some(Keycode::X), .. } => {
            gb.set_joypad(0, 0);
        }

        // KeyUp
        Event::KeyUp { keycode: Some(Keycode::Down), .. } => {
            gb.unset_joypad(1, 3);
        }
        Event::KeyUp { keycode: Some(Keycode::Up), .. } => {
            gb.unset_joypad(1, 2);
        }
        Event::KeyUp { keycode: Some(Keycode::Left), .. } => {
            gb.unset_joypad(1, 1);
        }
        Event::KeyUp { keycode: Some(Keycode::Right), .. } => {
            gb.unset_joypad(1, 0);
        }
        Event::KeyUp { keycode: Some(Keycode::V), .. } => {
            gb.unset_joypad(0, 3);
        }
        Event::KeyUp { keycode: Some(Keycode::C), .. } => {
            gb.unset_joypad(0, 2);
        }
        Event::KeyUp { keycode: Some(Keycode::Z), .. } => {
            gb.unset_joypad(0, 1);
        }
        Event::KeyUp { keycode: Some(Keycode::X), .. } => {
            gb.unset_joypad(0, 0);
        }
        Event::KeyUp { keycode: Some(Keycode::Escape), .. } => {
            //debugger_set_state(0);
        }
        _ => {}
    }
    false
}

fn find_sdl_gl_driver() -> Option<u32> {
    for (index, item) in sdl2::render::drivers().enumerate() {
        if item.name == "opengl" {
            return Some(index as u32);
        }
    }
    None
}

fn main() {
    let mut cycles: u32 = 0;
    let mut buffer: [u16; 256 * 256];
    let mut redraw = false;

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: ./mcugb romfile");
        return ();
    }

    let rom_path = &args[1];

    //mem_rom_headers();
    //system_load_rom_bank(1);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("mcugb", 320, 288)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    let mut canvas = window
        .into_canvas()
        .index(find_sdl_gl_driver().unwrap())
        .build()
        .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut gb = gb::GB::with_rom(&rom_path);
    gb.reset();

    println!("ROM Title: {:?}", gb.rom_title);

    'running: loop {
        for event in event_pump.poll_iter() {
            if handle_event(&event, &mut gb) {
                break 'running
            }
        }

        gb.step();

        canvas.clear();
        canvas.present();
        sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    /*
    if(pthread_create(&debugger_thread, 0, debugger_main, (void *) &RUNNING)) {
        printf("Unable to start debugger thread");
        return 1;
    }
    RUNNING = 1;

    while (1) {
        if (RUNNING) {
            if (STEP) {
                printf("Stepped one instruction, now at $%04hx.\n", REG_PC);
                cpu_debug();
                cpu_debug_stack();
                STEP = 0;
                debugger_set_state(0);
            } else if (debugger_in_breakpoints(REG_PC)) {
                printf("Triggered breakpoint at $%04hx.\n", REG_PC);
                cpu_debug();
                cpu_debug_stack();
                debugger_set_state(0);
            } else {
                cycles = cpu_step();
                redraw = gpu_step(cycles, buffer);
                //cpu_debug();
            }
        }

        /* draw buffer */
        if (redraw == 1 || RUNNING == 0) {
            if (SDL_PollEvent(&Event)) {
                if (handle_event(&Event)) break;
            }

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
            SDL_Delay(12);
        }
    }

    cpu_debug();
    mem_debug(0x9000, 128);
    mem_debug(0x9800, 128);
    printf("\n");
    mem_debug(0xff40, 16);
    printf("\n");
    mem_debug(0xfe00, 128);
    mem_debug(0xc0a4, 16);
    mem_debug(0x98a0, 16);
    printf("LCDC: $%04x\n", REG_LCDC);
    printf("STAT: $%04x\n", REG_STAT);
    printf("REG IE: $%04x\n", REG_INTERRUPT_ENABLE);
    printf("REG IF: $%04x\n", REG_INTERRUPT_FLAG);
    cpu_debug_stack();

    SDL_DestroyTexture(Texture);
    SDL_DestroyRenderer(Renderer);
    SDL_DestroyWindow(Window);
    SDL_Quit();
    */
}
