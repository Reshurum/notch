use super::interconnect::Interconnect;
use super::interconnect::END_RESERVED;

const INSTRUCTION_SIZE: u16 = 2;

#[derive(Default, Debug)]
pub struct Cpu {
    // Interconnect is used to control system resources like rom and memory.
    interconnect: Interconnect,

    // Program counter.
    pc: u16,

    // The function call stack.
    stack: [u16; 16],

    // Stack pointer.
    sp: u8,

    // General purpose registers v0-vf.
    v0: u8,
    v1: u8,
    v2: u8,
    v3: u8,
    v4: u8,
    v5: u8,
    v6: u8,
    v7: u8,
    v8: u8,
    v9: u8,
    va: u8,
    vb: u8,
    vc: u8,
    vd: u8,
    ve: u8,
    vf: u8,

    // Address register "I".
    i: u16,

    // Timer and sound registers.
    dt: u8,
    st: u8,
}

impl Cpu {
    pub fn new(interconnect: Interconnect) -> Cpu {
        Cpu {
            interconnect: interconnect,
            pc: END_RESERVED as u16,
            ..Cpu::default()
        }
    }

    /// Execute instructions from ram.
    pub fn run(&mut self) {
        loop {
            let word = self.interconnect.read_word(self.pc);

            // Execute until the subroutine ends.
            if self.execute_instruction(word) {
                break
            }
        }
    }

    #[inline(always)]
    fn execute_instruction(&mut self, instr: u16) -> bool {
        let opcode = (instr >> 12) as u8;

        //println!("{:#x}", instr);

        match opcode {
            0x6 => {
                // 6XNN - LD VX, NN
                //
                // Sets VX to NN.

                let regx = ((instr << 4) >> 12) as u8;
                let byte = ((instr << 8) >> 8) as u8;
                self.set_reg(regx, byte);
            },
            0xa => {
                // ANNN - LD I, NNN
                //
                // Sets I to the address NNN.

                let addr = ((instr << 4) >> 4) as u16;
                self.i = addr;
            },
            0xd => {
                // DXYN - DRW VX, VY, N
                //
                // Sprites stored in memory at location in index register (I),
                // 8bits wide. Wraps around the screen. If when drawn, clears a
                // pixel, register VF is set to 1 otherwise it is zero.
                //
                // All drawing is XOR drawing (i.e. it toggles the screen
                // pixels). Sprites are drawn starting at position VX, VY. N is
                // the number of 8bit rows that need to be drawn. If N is
                // greater than 1, second line continues at position VX, VY+1,
                // and so on.

                let regx = ((instr << 4) >> 12) as u8;
                let regy = ((instr << 8) >> 12) as u8;
                let nibble = ((instr << 12) >> 12) as usize;

                // Read N (nibble) bytes out out of memory starting at address
                // register I into our sprite.
                let mut sprite = vec![0 as u8; nibble];
                for i in 0..nibble {
                    sprite[i] = self.interconnect.ram[self.i as usize + i];
                }

                // Get screen coordinates from the requested registers.
                let x = self.get_reg(regx);
                let y = self.get_reg(regy);

                // Draw the sprite and store collision detection results in vf.
                self.vf = self.interconnect.draw(x as usize, y as usize, sprite);
            },
            0x2 => {
                // 2NNN - CALL NNN
                //
                // Calls subroutine at NNN.

                let addr = ((instr << 4) >> 4) as u16;

                // Add the current program counter to the call stack.
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;

                // Set the program counter to the call address begin executing
                // the subroutine.
                self.pc = addr;
                self.run();

                panic!("unhandled");
            },
            0xf => {
                let regx = ((instr << 4) >> 12) as u8;
                let identifier = ((instr << 8) >> 8) as u8;

                match identifier {
                    0x33 => {
                        // FX33 - LD B, VX
                        //
                        // Stores the Binary-coded decimal representation of VX,
                        // with the most significant of three digits at the
                        // address in I, the middle digit at I plus 1, and the
                        // least significant digit at I plus 2. (In other words,
                        // take the decimal representation of VX, place the
                        // hundreds digit in memory at location in I, the tens
                        // digit at location I+1, and the ones digit at
                        // location I+2.)

                        const DECIMAL_LENGTH: usize = 3;

                        let mut x: u8 = self.get_reg(regx);
                        let mut digits = vec![0 as u8; DECIMAL_LENGTH];
                        let mut digit_count: usize = 0;

                        // Organize the digits in the decimal into a slice.
                        while x > 0 {
                            digit_count += 1;
                            digits[DECIMAL_LENGTH - digit_count] = x % 10;
                            x /= 10;
                        }

                        // Set I, I+1, and I+3 to the values of the digits.
                        let i: usize = self.i as usize;
                        self.interconnect.ram[i] = digits[0];
                        self.interconnect.ram[i + 1] = digits[1];
                        self.interconnect.ram[i + 2] = digits[2];
                    },
                    _ => {
                        panic!("Found unknown identifier at instruction: {:#x}", instr);
                    }
                }
            },
            _ => {
                println!("cpu: {:#?}", self);
                panic!("Found unknown opcode at instruction: {:#x}", instr);
            }
        }

        // Increment the program counter to the next instruction.
        self.pc += INSTRUCTION_SIZE;

        false
    }

    /// Gets the value at a specified register.
    fn get_reg(&mut self, reg: u8) -> u8 {
        match reg {
            0x0 => self.v0,
            0x1 => self.v1,
            0x2 => self.v2,
            0x3 => self.v3,
            0x4 => self.v4,
            0x5 => self.v5,
            0x6 => self.v6,
            0x7 => self.v7,
            0x8 => self.v8,
            0x9 => self.v9,
            0xa => self.va,
            0xb => self.vb,
            0xc => self.vc,
            0xd => self.vd,
            0xe => self.ve,
            0xf => self.vf,
            _ => {
                panic!("Cannot get unknown register: V{:X}", reg);
            }
        }
    }

    /// Sets the value of a general purpose register.
    fn set_reg(&mut self, reg: u8, byte: u8) {
        match reg {
            0x0 => self.v0 = byte,
            0x1 => self.v1 = byte,
            0x2 => self.v2 = byte,
            0x3 => self.v3 = byte,
            0x4 => self.v4 = byte,
            0x5 => self.v5 = byte,
            0x6 => self.v6 = byte,
            0x7 => self.v7 = byte,
            0x8 => self.v8 = byte,
            0x9 => self.v9 = byte,
            0xa => self.va = byte,
            0xb => self.vb = byte,
            0xc => self.vc = byte,
            0xd => self.vd = byte,
            0xe => self.ve = byte,
            0xf => self.vf = byte,
            _ => {
                panic!("Cannot set unknown register: V{:X}", reg);
            }
        }
    }
}
