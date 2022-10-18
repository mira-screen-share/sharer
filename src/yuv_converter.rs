use std::num::Wrapping;

pub struct BGR0YUVConverter {
    yuv: Vec<u8>,
    width: usize,
    height: usize,
}

impl BGR0YUVConverter {
    /// Allocates a new helper for the given format.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            yuv: vec![0u8; (3 * (width * height)) / 2],
            width,
            height,
        }
    }

    /// Converts the RGB array.
    pub fn convert(&mut self, rgb: &[u8]) {
            let image_size = self.width * self.height as usize;
            let mut upos = image_size;
            let mut vpos = upos + upos / 4_usize;
            let mut i = 0_usize;

            for  line in 0..self.height {
                if line % 2 != 0 {
                    let mut x = 0_usize;
                    while x < self.width {
                        let b = Wrapping(rgb[4 * i] as u32);
                        let g = Wrapping(rgb[4 * i + 1] as u32);
                        let r = Wrapping(rgb[4 * i + 2] as u32);

                        self.yuv[i] = (((Wrapping(66)*r + Wrapping(129)*g + Wrapping(25)*b) >> 8) + Wrapping(16)).0 as u8;
                        self.yuv[upos] = (((Wrapping(-38i8 as u32)*r + Wrapping(-74i8 as u32)*g + Wrapping(112)*b) >> 8) + Wrapping(128)).0 as u8;
                        self.yuv[vpos] = (((Wrapping(112)*r + Wrapping(-94i8 as u32)*g + Wrapping(-18i8 as u32)*b) >> 8) + Wrapping(128)).0 as u8;

                        i+=1;
                        upos+=1;
                        vpos+=1;

                        let b = Wrapping(rgb[4 * i] as u32);
                        let g = Wrapping(rgb[4 * i + 1] as u32);
                        let r = Wrapping(rgb[4 * i + 2] as u32);

                        self.yuv[i] = (((Wrapping(66)*r + Wrapping(129)*g + Wrapping(25)*b) >> 8) + Wrapping(16)).0 as u8;
                        i+=1;
                        x+=2;
                    }
                } else {
                    for _x in 0..self.width{
                        let b = Wrapping(rgb[4 * i] as u32);
                        let g = Wrapping(rgb[4 * i + 1] as u32);
                        let r = Wrapping(rgb[4 * i + 2] as u32);

                        self.yuv[i] = (((Wrapping(66)*r + Wrapping(129)*g + Wrapping(25)*b) >> 8) + Wrapping(16)).0 as u8;
                        i+=1;
                    }
                }
            }
    }

    fn width(&self) -> i32 {
        self.width as i32
    }

    fn height(&self) -> i32 {
        self.height as i32
    }

    pub fn y(&self) -> &[u8] {
        &self.yuv[0..self.width * self.height]
    }

    pub fn u(&self) -> &[u8] {
        let base_u = self.width * self.height;
        &self.yuv[base_u..base_u + base_u / 4]
    }

    pub fn v(&self) -> &[u8] {
        let base_u = self.width * self.height;
        let base_v = base_u + base_u / 4;
        &self.yuv[base_v..]
    }
}

fn clamp(val: i32) -> u8 {
    match val {
        ref v if *v < 0 => 0,
        ref v if *v > 255 => 255,
        v => v as u8,
    }
}