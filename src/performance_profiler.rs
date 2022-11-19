use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{Timelike, Utc};
// use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

pub struct PerformanceProfiler {
    frame_time: i64,
    pre_processing_time: i64,
    conversion_time: i64,
    encoding_time: i64,
    total_time: i64,
    counter_frequency: i64,
    last_second: u8,
    last_second_frame_count: u8,
    current_second_frame_count: u8,
    log_enabled: bool,
    bytes_encoded: usize,
    last_bitrate: f64, // in kbps
    max_fps: u32,
}

impl PerformanceProfiler {
    pub fn new(log_enabled: bool, max_fps: u32) -> Self {
        Self {
            frame_time: 0,
            pre_processing_time: 0,
            conversion_time: 0,
            encoding_time: 0,
            total_time: 0,
            counter_frequency: Self::get_qp_frequency(),
            last_second: 0,
            last_second_frame_count: 0,
            current_second_frame_count: 0,
            bytes_encoded: 0,
            log_enabled,
            last_bitrate: 0.0,
            max_fps,
        }
    }

    pub fn accept_frame(&mut self, _frame_time: i64) {
        self.frame_time = Self::get_qp_counter(); // frame_time is not accurate
    }

    pub fn done_preprocessing(&mut self) {
        self.pre_processing_time = Self::get_qp_counter();
    }

    pub fn done_conversion(&mut self) {
        self.conversion_time = Self::get_qp_counter();
    }

    pub fn done_encoding(&mut self) {
        self.encoding_time = Self::get_qp_counter();
    }

    pub fn done_processing(&mut self, size: usize) {
        self.total_time = Self::get_qp_counter();
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
            / self.counter_frequency as f64
            * 1000.0;
        let conversion_time = (self.conversion_time - self.pre_processing_time) as f64
            / self.counter_frequency as f64
            * 1000.0;
        let encoding_time = (self.encoding_time - self.conversion_time) as f64
            / self.counter_frequency as f64
            * 1000.0;
        let webrtc_time =
            (self.total_time - self.encoding_time) as f64 / self.counter_frequency as f64 * 1000.0;
        let total_time =
            (self.total_time - self.frame_time) as f64 / self.counter_frequency as f64 * 1000.0;

        info!(
            "Total time {:.1}ms ({:.1} p, {:.1} c, {:.1} e, {:.1} s) {:.1}% at {} FPS. Current FPS: {}/{:.1}. {:.1} kbps",
            total_time,
            pre_processing_time,
            conversion_time,
            encoding_time,
            webrtc_time,
            (total_time / (1.0 / self.max_fps as f64 * 1000.0)) * 100.0,
            self.max_fps,
            self.last_second_frame_count,
            1000.0/total_time as f64,
            self.last_bitrate
        );
    }

    fn get_qp_counter() -> i64 {
        let e = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        (e.as_secs() * 1000 + e.subsec_nanos() as u64 / 1_000_000) as i64
    }
    fn get_qp_frequency() -> i64 {
        1000
    }
}
