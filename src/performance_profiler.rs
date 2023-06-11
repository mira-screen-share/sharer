use chrono::{Timelike, Utc};
use howlong::HighResolutionTimer;

pub struct PerformanceProfiler {
    frame_time: u128,
    pre_processing_time: u128,
    encoding_time: u128,
    total_time: u128,
    counter_conv_to_s: u128,
    last_second: u8,
    last_second_frame_count: u8,
    current_second_frame_count: u8,
    log_enabled: bool,
    bytes_encoded: usize,
    last_bitrate: f64, // in kbps
    max_fps: u32,
    timer: HighResolutionTimer,
}

impl PerformanceProfiler {
    pub fn new(log_enabled: bool, max_fps: u32) -> Self {
        Self {
            frame_time: 0,
            pre_processing_time: 0,
            encoding_time: 0,
            total_time: 0,
            counter_conv_to_s: 1_000_000,
            last_second: 0,
            last_second_frame_count: 0,
            current_second_frame_count: 0,
            bytes_encoded: 0,
            log_enabled,
            last_bitrate: 0.0,
            max_fps,
            timer: HighResolutionTimer::new(),
        }
    }

    pub fn accept_frame(&mut self, _frame_time: i64) {
        self.frame_time = self.current_time(); // frame_time is not accurate
    }

    pub fn done_preprocessing(&mut self) {
        self.pre_processing_time = self.current_time();
    }

    pub fn done_encoding(&mut self) {
        self.encoding_time = self.current_time();
    }

    pub fn done_processing(&mut self, size: usize) {
        self.total_time = self.current_time();
        let current_second = Utc::now().second() as u8;
        if current_second != self.last_second {
            self.last_second = current_second;
            self.last_second_frame_count = self.current_second_frame_count;
            self.last_bitrate = self.bytes_encoded as f64 * 8.0 / 1000.0;
            self.current_second_frame_count = 1;
            self.bytes_encoded = size;
        } else {
            self.current_second_frame_count += 1;
            self.bytes_encoded += size;
        }
        if self.log_enabled {
            self.report();
        }
    }

    fn report(&self) {
        let pre_processing_time = (self.pre_processing_time - self.frame_time) as f64
            / self.counter_conv_to_s as f64
            * 1000.0;
        let encoding_time = (self.encoding_time - self.pre_processing_time) as f64
            / self.counter_conv_to_s as f64
            * 1000.0;
        let webrtc_time =
            (self.total_time - self.encoding_time) as f64 / self.counter_conv_to_s as f64 * 1000.0;
        let total_time =
            (self.total_time - self.frame_time) as f64 / self.counter_conv_to_s as f64 * 1000.0;
        if webrtc_time > 8. {
            warn!("send time abnormal: {}", webrtc_time);
        }

        info!(
            "Total time {:.1}ms ({:.1} p, {:.1} e, {:.1} s) {:.1}% at {} FPS. Current FPS: {}/{:.1}. {:.1} kbps",
            total_time,
            pre_processing_time,
            encoding_time,
            webrtc_time,
            (total_time / (1.0 / self.max_fps as f64 * 1000.0)) * 100.0,
            self.max_fps,
            self.last_second_frame_count,
            1000.0/total_time,
            self.last_bitrate
        );
    }

    fn current_time(&self) -> u128 {
        self.timer.elapsed().as_micros()
    }
}
