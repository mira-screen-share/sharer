use ac_ffmpeg::codec::audio::frame::get_sample_format;
use ac_ffmpeg::codec::audio::{AudioFrameMut, SampleFormat};
use apple_sys::AVFAudio::{
    AVAudioPCMBuffer, IAVAudioBuffer, IAVAudioFormat, IAVAudioPCMBuffer, PNSObject,
};

pub enum PCMData {
    F32(Vec<f32>),
    I16(Vec<i16>),
    I32(Vec<i32>),
}

impl PCMData {
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            PCMData::F32(data) => unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr() as *const u8,
                    data.len() * std::mem::size_of::<f32>(),
                )
                .to_vec()
            },
            PCMData::I16(data) => unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr() as *const u8,
                    data.len() * std::mem::size_of::<i16>(),
                )
                .to_vec()
            },
            PCMData::I32(data) => unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr() as *const u8,
                    data.len() * std::mem::size_of::<i32>(),
                )
                .to_vec()
            },
        }
    }
}

pub struct PCMBuffer {
    buffer: AVAudioPCMBuffer,
    pub data: PCMData,
    pub sample_rate: f64,
    pub channels: usize,
    pub sample_size: usize,
    pub stride: usize,
}

impl PCMBuffer {
    unsafe fn aaa<T: Copy + Default>(
        stride: usize,
        channels: usize,
        sample_size: usize,
        data: *const *mut T,
    ) -> Vec<T> {
        let mut ret: Vec<T> = Vec::with_capacity(channels * sample_size);
        if stride > 1 || channels == 1 {
            for i in 0..(sample_size * channels) {
                ret.push(data.read().add(i).read());
            }
        } else {
            let channel_data = std::slice::from_raw_parts(data, channels);
            for j in 0..sample_size {
                for i in 0..channels {
                    ret.push(channel_data[i].add(j).read());
                }
            }
        }
        ret
    }

    pub fn new(buffer: AVAudioPCMBuffer) -> Self {
        unsafe {
            let stride = buffer.stride() as usize;
            let channels = buffer.format().channelCount() as usize;
            let sample_size = buffer.frameLength() as usize;
            let data = if !buffer.floatChannelData().is_null() {
                PCMData::F32(Self::aaa::<f32>(
                    stride,
                    channels,
                    sample_size,
                    buffer.floatChannelData(),
                ))
            } else if !buffer.int16ChannelData().is_null() {
                PCMData::I16(Self::aaa::<i16>(
                    stride,
                    channels,
                    sample_size,
                    buffer.int16ChannelData(),
                ))
            } else if !buffer.int32ChannelData().is_null() {
                PCMData::I32(Self::aaa::<i32>(
                    stride,
                    channels,
                    sample_size,
                    buffer.int32ChannelData(),
                ))
            } else {
                panic!("Unreachable");
            };
            Self {
                buffer,
                data,
                sample_rate: buffer.format().sampleRate() as _,
                channels,
                sample_size,
                stride,
            }
        }
    }

    pub fn sample_format(&self) -> SampleFormat {
        get_sample_format(match self.data {
            PCMData::F32(_) => "flt",
            PCMData::I16(_) => "s16",
            PCMData::I32(_) => "s32",
        })
    }

    fn get_to_sample_slice<T>(&self, data: &[u8]) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(
                data.as_ptr() as *mut T,
                data.len() / std::mem::size_of::<T>(),
            )
        }
    }

    pub fn write_samples_into(&self, frame: &mut AudioFrameMut) {
        let plane = &mut frame.planes_mut()[0];
        let data = plane.data_mut();
        match &self.data {
            PCMData::F32(from_samples) => {
                let to_samples: &mut [f32] = self.get_to_sample_slice(data);
                to_samples[..from_samples.len()].copy_from_slice(from_samples);
            }
            PCMData::I16(from_samples) => {
                let to_samples: &mut [i16] = self.get_to_sample_slice(data);
                to_samples[..from_samples.len()].copy_from_slice(from_samples);
            }
            PCMData::I32(from_samples) => {
                let to_samples: &mut [i32] = self.get_to_sample_slice(data);
                to_samples[..from_samples.len()].copy_from_slice(from_samples);
            }
        }
    }
}

unsafe impl Send for PCMBuffer {}

impl Drop for PCMBuffer {
    fn drop(&mut self) {
        unsafe {
            self.buffer.release();
        }
    }
}
