extern crate sdl2;

mod cpu;
mod gb;
mod gpu;
mod memory;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::env;
use std::collections::HashSet;
use sdl2::rect::Rect;

static SCALE_FACTOR: u32 = 2;

fn handle_event(event: &Event, gb: &mut gb::GB) -> bool {
    match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => return true,

        // KeyDown
        Event::KeyDown {
            keycode: Some(Keycode::Down),
            ..
        } => {
            gb.set_joypad(1, 3);
        }
        Event::KeyDown {
            keycode: Some(Keycode::Up),
            ..
        } => {
            gb.set_joypad(1, 2);
        }
        Event::KeyDown {
            keycode: Some(Keycode::Left),
            ..
        } => {
            gb.set_joypad(1, 1);
        }
        Event::KeyDown {
            keycode: Some(Keycode::Right),
            ..
        } => {
            gb.set_joypad(1, 0);
        }
        Event::KeyDown {
            keycode: Some(Keycode::V),
            ..
        } => {
            gb.set_joypad(0, 3);
        }
        Event::KeyDown {
            keycode: Some(Keycode::C),
            ..
        } => {
            gb.set_joypad(0, 2);
        }
        Event::KeyDown {
            keycode: Some(Keycode::Z),
            ..
        } => {
            gb.set_joypad(0, 1);
        }
        Event::KeyDown {
            keycode: Some(Keycode::X),
            ..
        } => {
            gb.set_joypad(0, 0);
        }

        // KeyUp
        Event::KeyUp {
            keycode: Some(Keycode::Down),
            ..
        } => {
            gb.unset_joypad(1, 3);
        }
        Event::KeyUp {
            keycode: Some(Keycode::Up),
            ..
        } => {
            gb.unset_joypad(1, 2);
        }
        Event::KeyUp {
            keycode: Some(Keycode::Left),
            ..
        } => {
            gb.unset_joypad(1, 1);
        }
        Event::KeyUp {
            keycode: Some(Keycode::Right),
            ..
        } => {
            gb.unset_joypad(1, 0);
        }
        Event::KeyUp {
            keycode: Some(Keycode::V),
            ..
        } => {
            gb.unset_joypad(0, 3);
        }
        Event::KeyUp {
            keycode: Some(Keycode::C),
            ..
        } => {
            gb.unset_joypad(0, 2);
        }
        Event::KeyUp {
            keycode: Some(Keycode::Z),
            ..
        } => {
            gb.unset_joypad(0, 1);
        }
        Event::KeyUp {
            keycode: Some(Keycode::X),
            ..
        } => {
            gb.unset_joypad(0, 0);
        }
        Event::KeyUp {
            keycode: Some(Keycode::Escape),
            ..
        } => {
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
    let mut break_points: HashSet<u16> = vec![
        // for testing
     ].into_iter().collect();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: ./mcugb romfile");
        return ();
    }

    let rom_path = &args[1];

    // parse command line breakpoints
    break_points.extend(
        env::args().skip(2).map(|arg| u16::from_str_radix(&arg, 16).unwrap())
    );

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("mcugb", 160 * SCALE_FACTOR, 144 * SCALE_FACTOR)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    let mut canvas = window
        .into_canvas()
        .index(find_sdl_gl_driver().unwrap())
        .build()
        .unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_target(
        PixelFormatEnum::RGBA8888,
        256, 256
    ).unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut gb = gb::GB::with_rom(&rom_path);
    gb.reset();

    println!("ROM Title: {:?}", gb.rom_title);
    let mut buf: [u8; 256 * 256 * 4] = [0; 256 * 256 * 4];
    'running: loop {
        for event in event_pump.poll_iter() {
            if handle_event(&event, &mut gb) {
                break 'running;
            }
        }

        if break_points.contains(&gb.cpu.pc) {
            eprintln!("!!! Hit break point at {:04X}", gb.cpu.pc);
            break 'running;
        }

        let (_cycles, redraw) = gb.step(&mut buf);

        if redraw {
            canvas.clear();
            texture.update(Rect::new(0, 0, 256, 256), &buf, 256 * 4).unwrap();
            let wx = *gb.mem.reg_wx() as i32;
            let wy = *gb.mem.reg_wy() as i32;
            canvas.copy(&texture, Rect::new(wx, wy, 160, 144), None).unwrap();
            canvas.present();
        }
    }

    dump_debug(&gb);
    dump_mem(&gb, 0xffb0);
}

fn dump_debug(gb: &gb::GB) {
    println!("");

    // registers
    println!("CPU Registers:");
    println!("A: {:02X}    F: {:02X}", gb.cpu.reg.a, gb.cpu.reg.f);
    println!("B: {:02X}    C: {:02X}", gb.cpu.reg.b, gb.cpu.reg.c);
    println!("D: {:02X}    E: {:02X}", gb.cpu.reg.d, gb.cpu.reg.e);
    println!("H: {:02X}    L: {:02X}", gb.cpu.reg.h, gb.cpu.reg.l);
    println!("PC: {:04X}", gb.cpu.pc);
    println!("SP: {:04X}", gb.cpu.sp);

    println!("");

    // stack
    println!("Stack");
    for i in 0..16 {
        let address = ((i * 2) + gb.cpu.sp - 16) as usize;
        println!("{:04X} | {:02X}{:02X}", address, gb.mem.data[address+1], gb.mem.data[address]);
    }
}

fn dump_mem(gb: &gb::GB, address: u16) {
    println!("");

    println!("Mem at 0x{:04X}", address);
    for r in 0..8 {
        let row_address = (r * 8 + address) as usize;
        print!("0x{:04X} | ", row_address);
        for c in 0..8 {
            let byte_address = (row_address + c)  as usize;
            print!("{:02X} ", gb.mem.data[byte_address]);
        }
        println!("");
    }
}