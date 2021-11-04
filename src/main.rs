mod arraystack;
use arraystack::Stack;

use std::time::{Duration, Instant};

use anyhow::{bail, Context};
use gumdrop::Options;
use minifb::{Key, Window, WindowOptions};

const MEM_SIZE: usize = 4096;
const STACK_SIZE: usize = 16;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

const INSTRUCTIONS_PER_SEC: u32 = 700;
const TIMER_UPDATE_RATE_HZ: u32 = 60;

const PROGRAM_START: usize = 0x200;

const FONT_START: usize = 0x050;
const FONT_END: usize = 0x0A0;
const FONT_CHAR_SIZE_BYTES: usize = 5;
const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

const KEY_MAP: [Key; 16] = [
    Key::X,
    Key::Key1,
    Key::Key2,
    Key::Key3,
    Key::Q,
    Key::W,
    Key::E,
    Key::A,
    Key::S,
    Key::D,
    Key::Z,
    Key::C,
    Key::Key4,
    Key::R,
    Key::F,
    Key::V,
];

struct State {
    memory: [u8; MEM_SIZE],
    program_counter: u16,
    index_register: u16,
    stack: Stack<u16, STACK_SIZE>,
    variable_registers: [u8; 16],
    delay_timer: u8,
    sound_timer: u8,
    display: [bool; WIDTH * HEIGHT],
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("program_counter", &self.program_counter)
            .field("index_register", &self.index_register)
            .field("delay_timer", &self.delay_timer)
            .field("sound_timer", &self.sound_timer)
            .field("stack", &self.stack)
            .field("variable_registers", &self.variable_registers)
            .finish()
    }
}

#[derive(Options)]
struct Args {
    #[options(free, required, help = "Path to the CHIP-8 ROM")]
    filename: String,

    #[options(
        no_short,
        help = "Shift instruction uses VX without first setting VX to VY"
    )]
    bitshift_ignores_vy: bool,
    #[options(no_short, help = "Jump-with-offset adds VX instead of V0")]
    jump_with_offset_uses_vx: bool,
    #[options(
        no_short,
        help = "Add to index instruction does not set VF on overflow"
    )]
    add_to_index_ignores_overflow: bool,
    #[options(no_short, help = "Increment index register by X after store and load")]
    store_and_load_increment_index: bool,

    #[options(help = "Print help message")]
    help: bool,
}

// TODO copy_from_slice() may panic. Find a way to address.
fn main() -> anyhow::Result<()> {
    let args: Args = gumdrop::parse_args_default_or_exit();

    let mut state = {
        let mut memory = [0; MEM_SIZE];
        memory[FONT_START..FONT_END].copy_from_slice(&FONT);

        State {
            memory,
            program_counter: 0x200,
            index_register: 0,
            stack: Stack::new(),
            variable_registers: [0; 16],
            delay_timer: 0,
            sound_timer: 0,
            display: [false; WIDTH * HEIGHT],
        }
    };

    let program = std::fs::read(&args.filename).context("Couldn't read program file")?;
    state.memory[PROGRAM_START..][..program.len()].copy_from_slice(program.as_slice());

    let mut display_buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "CHIP-8 Emulator",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: minifb::Scale::X8,
            ..Default::default()
        },
    )
    .context("Couldn't create window")?;

    window.limit_update_rate(Some(Duration::from_secs(1) / TIMER_UPDATE_RATE_HZ));

    let mut last_render = Instant::now();
    let mut processing_time = Duration::ZERO;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        processing_time += last_render.elapsed();
        last_render = Instant::now();

        while let Some(remaining_time) =
            processing_time.checked_sub(Duration::from_secs(1) / INSTRUCTIONS_PER_SEC)
        {
            processing_time = remaining_time;

            let [instruction_hi, instruction_lo]: [u8; 2] = state.memory
                [state.program_counter as usize..][..2]
                .try_into()
                .context("Out-of-bounds while trying to fetch instruction")?;
            state.program_counter += 2;

            let instruction = u16::from_be_bytes([instruction_hi, instruction_lo]);

            let [[nibble1, nibble2], [nibble3, nibble4]] =
                instruction.to_be_bytes().map(|b| [b >> 4, b & 0x0F]);
            let nibbles = [nibble1, nibble2, nibble3, nibble4];
            let nnn = instruction & 0x0FFF;
            let nn = instruction_lo;

            match nibbles {
                [0x0, 0x0, 0xE, 0x0] => state.display.fill(false),
                [0x0, 0x0, 0xE, 0xE] => {
                    state.program_counter =
                        state.stack.pop().context("Tried to pop from empty stack")?
                }
                [0x1, ..] => state.program_counter = nnn,
                [0x2, ..] => {
                    state
                        .stack
                        .try_push(state.program_counter)
                        .context("Overflowed stack")?;
                    state.program_counter = nnn;
                }
                [0x3, x, ..] => {
                    if state.variable_registers[x as usize] == nn {
                        state.program_counter += 2;
                    }
                }
                [0x4, x, ..] => {
                    if state.variable_registers[x as usize] != nn {
                        state.program_counter += 2;
                    }
                }
                [0x5, x, y, 0x0] => {
                    if state.variable_registers[x as usize] == state.variable_registers[y as usize]
                    {
                        state.program_counter += 2;
                    }
                }
                [0x6, x, ..] => state.variable_registers[x as usize] = nn,
                [0x7, x, ..] => {
                    let vx = &mut state.variable_registers[x as usize];
                    *vx = vx.wrapping_add(nn);
                }
                [0x8, x, y, instr] => {
                    // We assign VF last if VX and VF overlap.
                    // Due to borrow checker, we'll reference VF through state.variable_registers[0xF]
                    // after we're done operating on VX + VY
                    let vy = state.variable_registers[y as usize];
                    let vx = &mut state.variable_registers[x as usize];

                    match instr {
                        0x0 => *vx = vy,
                        0x1 => *vx |= vy,
                        0x2 => *vx &= vy,
                        0x3 => *vx ^= vy,
                        0x4 => {
                            let (sum, overflowed) = vx.overflowing_add(vy);
                            *vx = sum;
                            state.variable_registers[0xF] = if overflowed { 1 } else { 0 };
                        }
                        0x5 => {
                            let (difference, overflowed) = vx.overflowing_sub(vy);
                            *vx = difference;
                            // Note that CHIP-8 sets VF to 1 only if we didn't underflow
                            state.variable_registers[0xF] = if overflowed { 0 } else { 1 };
                        }
                        0x6 => {
                            if !args.bitshift_ignores_vy {
                                *vx = vy;
                            }
                            let (result, overflowed) = vx.overflowing_shr(1);
                            *vx = result;
                            state.variable_registers[0xF] = if overflowed { 1 } else { 0 };
                        }
                        0x7 => {
                            let (difference, overflowed) = vy.overflowing_sub(*vx);
                            *vx = difference;
                            // Note that CHIP-8 sets VF to 1 only if we didn't underflow
                            state.variable_registers[0xF] = if overflowed { 0 } else { 1 };
                        }
                        0xE => {
                            if !args.bitshift_ignores_vy {
                                *vx = vy;
                            }
                            let (result, overflowed) = vx.overflowing_shl(1);
                            *vx = result;
                            state.variable_registers[0xF] = if overflowed { 1 } else { 0 };
                        }
                        _ => bail!("Unexpected arithmetic instruction: {:04x}", instruction),
                    }
                }
                [0x9, x, y, 0x0] => {
                    if state.variable_registers[x as usize] != state.variable_registers[y as usize]
                    {
                        state.program_counter += 2;
                    }
                }
                [0xA, ..] => state.index_register = nnn,
                [0xB, x, ..] => {
                    if args.jump_with_offset_uses_vx {
                        state.index_register = nnn + state.variable_registers[x as usize] as u16;
                    } else {
                        state.index_register = nnn + state.variable_registers[0] as u16;
                    }
                }
                [0xC, x, ..] => {
                    let random_num: u8 = rand::random();
                    state.variable_registers[x as usize] = random_num & nn;
                }
                [0xD, x, y, n] => {
                    let x_start = (state.variable_registers[x as usize] as usize) % WIDTH;
                    let y_start = (state.variable_registers[y as usize] as usize) % HEIGHT;

                    state.variable_registers[0xF] = 0;

                    // TODO Do this better
                    let sprite = &state.memory[(state.index_register as usize)..][..(n as usize)];
                    if sprite.len() != n as usize {
                        bail!("Out-of-bounds while trying to read sprite")
                    }

                    // TODO Lots of arithmetic here. Deal with overflow
                    for (sprite_row, y) in sprite.iter().zip(y_start..) {
                        if y >= HEIGHT {
                            break;
                        }

                        for (i, x) in (x_start..x_start.saturating_add(8)).enumerate() {
                            if x >= WIDTH {
                                break;
                            }

                            if (sprite_row >> (8 - i - 1)) & 0x01 == 1 {
                                let pixel = &mut state.display[y * WIDTH + x];
                                *pixel = !*pixel;

                                if !(*pixel) {
                                    state.variable_registers[0xF] = 1;
                                }
                            }
                        }
                    }
                }
                [0xE, x, 0x9, 0xE] => {
                    if window.is_key_down(KEY_MAP[state.variable_registers[x as usize] as usize]) {
                        state.program_counter += 2;
                    }
                }
                [0xE, x, 0xA, 0x1] => {
                    if !window.is_key_down(KEY_MAP[state.variable_registers[x as usize] as usize]) {
                        state.program_counter += 2;
                    }
                }
                [0xF, x, 0x0, 0x7] => state.variable_registers[x as usize] = state.delay_timer,
                [0xF, x, 0x0, 0xA] => {
                    match KEY_MAP
                        .iter()
                        .enumerate()
                        .find(|(_, key)| window.is_key_down(**key))
                    {
                        Some((i, _key)) => state.variable_registers[x as usize] = i as u8,
                        None => state.program_counter -= 2,
                    }
                }
                [0xF, x, 0x1, 0x5] => state.delay_timer = state.variable_registers[x as usize],
                [0xF, x, 0x1, 0x8] => state.sound_timer = state.variable_registers[x as usize],
                [0xF, x, 0x1, 0xE] => {
                    state.index_register += state.variable_registers[x as usize] as u16;
                    if args.add_to_index_ignores_overflow {
                        if state.index_register > 0xFFF {
                            state.variable_registers[0xF] = 1; // TODO Set to 0 otherwise?
                        }
                    }
                }
                [0xF, x, 0x2, 0x9] => {
                    let vx = state.variable_registers[x as usize];
                    state.index_register = (FONT_START + FONT_CHAR_SIZE_BYTES * vx as usize) as u16;
                }
                [0xF, x, 0x3, 0x3] => {
                    let vx = state.variable_registers[x as usize];
                    let [hundreds, tens, ones]: [u16; 3] =
                        [vx / 100, (vx / 10) % 10, vx % 10].map(u16::from);
                    state.index_register = (hundreds << 8) | (tens << 4) | ones;
                }
                [0xF, x, 0x5, 0x5] => {
                    let dst = state
                        .memory
                        .get_mut((state.index_register as usize)..)
                        .and_then(|range| range.get_mut(..=(x as usize)))
                        .context("Overflowed while trying to store registers V0 through VX")?;
                    let src = &state.variable_registers[..=(x as usize)];
                    dst.copy_from_slice(src);

                    if args.store_and_load_increment_index {
                        state.index_register += x as u16;
                    }
                }
                [0xF, x, 0x6, 0x5] => {
                    let dst = &mut state.variable_registers[..=(x as usize)];
                    let src = state
                        .memory
                        .get((state.index_register as usize)..)
                        .and_then(|range| range.get(..=(x as usize)))
                        .context("Overflowed while trying to store registers V0 through VX")?;
                    dst.copy_from_slice(src);

                    if args.store_and_load_increment_index {
                        state.index_register += x as u16;
                    }
                }
                _ => bail!("Unexpected instruction: {:04x}", instruction),
            }
        }

        // Update timers
        state.delay_timer = state.delay_timer.saturating_sub(1);
        state.sound_timer = state.sound_timer.saturating_sub(1);
        // TODO Handle sound timer

        // Render
        for (pixel, val) in display_buffer.iter_mut().zip(&state.display) {
            *pixel = if *val { !0 } else { 0 };
        }

        window
            .update_with_buffer(&display_buffer, WIDTH, HEIGHT)
            .context("Couldn't update window")?;
    }

    Ok(())
}
