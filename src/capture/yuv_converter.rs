use itertools::Itertools;
use std::num::Wrapping;

pub struct BGR0YUVConverter {
    width: usize,
    height: usize,
}

impl BGR0YUVConverter {
    /// Allocates a new helper for the given format.
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    /// Converts the RGB array.
    pub fn convert(&mut self, rgb: &[u8], yuv: Vec<&mut [u8]>) {
        let mut upos = 0_usize;
        let mut vpos = 0_usize;
        let mut i = 0_usize;
        let (y, u, v) = yuv.into_iter().tuples().next().unwrap();

        for line in 0..self.height {
            if line % 2 != 0 {
                let mut x = 0_usize;
                while x < self.width {
                    let b = Wrapping(rgb[4 * i] as u32);
                    let g = Wrapping(rgb[4 * i + 1] as u32);
                    let r = Wrapping(rgb[4 * i + 2] as u32);

                    y[i] = (((Wrapping(66) * r + Wrapping(129) * g + Wrapping(25) * b) >> 8)
                        + Wrapping(16))
                    .0 as u8;
                    u[upos] = (((Wrapping(-38i8 as u32) * r
                        + Wrapping(-74i8 as u32) * g
                        + Wrapping(112) * b)
                        >> 8)
                        + Wrapping(128))
                    .0 as u8;
                    v[vpos] = (((Wrapping(112) * r
                        + Wrapping(-94i8 as u32) * g
                        + Wrapping(-18i8 as u32) * b)
                        >> 8)
                        + Wrapping(128))
                    .0 as u8;

                    i += 1;
                    upos += 1;
                    vpos += 1;

                    let b = Wrapping(rgb[4 * i] as u32);
                    let g = Wrapping(rgb[4 * i + 1] as u32);
                    let r = Wrapping(rgb[4 * i + 2] as u32);

                    y[i] = (((Wrapping(66) * r + Wrapping(129) * g + Wrapping(25) * b) >> 8)
                        + Wrapping(16))
                    .0 as u8;
                    i += 1;
                    x += 2;
                }
            } else {
                for _x in 0..self.width {
                    let b = Wrapping(rgb[4 * i] as u32);
                    let g = Wrapping(rgb[4 * i + 1] as u32);
                    let r = Wrapping(rgb[4 * i + 2] as u32);

                    y[i] = (((Wrapping(66) * r + Wrapping(129) * g + Wrapping(25) * b) >> 8)
                        + Wrapping(16))
                    .0 as u8;
                    i += 1;
                }
            }
        }
    }
}
