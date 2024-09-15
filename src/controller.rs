use sdl2::controller;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Key {
    B,
    Y,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
    A,
    X,
    L,
    R,
}

#[derive(Default, Debug)]
pub struct Controller {
    pub data: [u16; 2],
    pos: usize,
    clk: bool,
}

impl Controller {
    pub fn initialize(&mut self) {
        self.pos = 0;
        // self.flag = false;
    }

    pub fn read(&mut self) -> u8 {
        let ret = if self.pos > 15 {
            0b0000_0011
        } else {
            let mut data = 0;
            if self.data[0] & (1 << (15 - self.pos)) != 0 {
                data |= 0b0000_0001;
            }
            if self.data[1] & (1 << (15 - self.pos)) != 0 {
                data |= 0b0000_0010;
            }

            // if self.flag {
            //     self.pos += 1;
            //     self.flag = false;
            // } else {
            //     self.flag = true;
            // }
            self.pos += 1;
            data
        };
        // if self.flag {
        //     self.pos += 1;
        //     self.flag = false;
        // } else {
        //     self.flag = true;
        // }

        ret
    }

    // pub fn controller_read(&mut self, pin: usize) -> bool {
    //     match pin {
    //         4 | 5 => {
    //             let i = pin - 4;
    //             if self.pos < 16 {
    //                 self.data[i] & (1 << (15 - self.pos)) != 0
    //             } else {
    //                 true
    //             }
    //         }
    //         6 => true,
    //         _ => unreachable!(),
    //     }
    // }

    // pub fn controller_write(&mut self, pin: usize, data: bool) {
    //     match pin {
    //         2 => {
    //             let prev = self.clk;
    //             self.clk = data;
    //             if !prev && self.clk {
    //                 self.pos += 1;
    //             }
    //         }
    //         3 => {
    //             if data {
    //                 self.pos = 0;
    //                 self.clk = false;
    //             }
    //         }
    //         6 => {}
    //         _ => unreachable!(),
    //     }
    // }
}
