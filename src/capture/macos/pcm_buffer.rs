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
    pub frame_duration: f64,
}

impl PCMBuffer {
    pub fn new(buffer: AVAudioPCMBuffer) -> Self {
        unsafe {
            let stride = buffer.stride() as usize;
            let channels = buffer.format().channelCount() as usize;
            let sample_size = buffer.frameLength() as usize;
            let sample_rate = buffer.format().sampleRate();
            let frame_duration = 1000.0 / (sample_rate / sample_size as f64);
            let data = if !buffer.floatChannelData().is_null() {
                PCMData::F32(read_buffer_data::<f32>(
                    buffer.floatChannelData(),
                    stride,
                    channels,
                    sample_size,
                ))
            } else if !buffer.int16ChannelData().is_null() {
                PCMData::I16(read_buffer_data::<i16>(
                    buffer.int16ChannelData(),
                    stride,
                    channels,
                    sample_size,
                ))
            } else if !buffer.int32ChannelData().is_null() {
                PCMData::I32(read_buffer_data::<i32>(
                    buffer.int32ChannelData(),
                    stride,
                    channels,
                    sample_size,
                ))
            } else {
                panic!("Unreachable");
            };
            Self {
                buffer,
                data,
                sample_rate,
                channels,
                sample_size,
                stride,
                frame_duration,
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

unsafe fn read_buffer_data<T: Copy + Default>(
    data: *const *mut T,
    stride: usize,
    channels: usize,
    sample_size: usize,
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

unsafe impl Send for PCMBuffer {}

impl Drop for PCMBuffer {
    fn drop(&mut self) {
        unsafe {
            self.buffer.release();
        }
    }
}
