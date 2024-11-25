pub(crate) mod js;

use atomic_float::AtomicF64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::buffer::Buffer;
use nih_plug::plugin::ProcessStatus;
use nih_plug::prelude::{BufferConfig, FloatParam};

pub(crate) trait Engine: Send + Sync {
    fn init(&mut self, buffer_config: &BufferConfig) -> bool;
    fn process(&mut self, buffer: &mut Buffer) -> ProcessStatus;

    fn t(&self) -> i64;
    fn reset_t(&self);

    fn set_func(&mut self, func: &str);
}

pub(self) struct EngineBase {
    frequency: Arc<FloatParam>,
    sample_rate: f32,

    // NOTE: Use float for accurate incrementation and atomic
    //       because it is shared with the editor.
    t: AtomicF64,
    ts: Vec<i64>,
    output: Vec<f32>,
}

impl EngineBase {
    pub(self) fn new(frequency: Arc<FloatParam>) -> Self {
        Self {
            frequency,
            sample_rate: 48000.0,

            t: AtomicF64::new(0.0),
            ts: Vec::new(),
            output: Vec::new(),
        }
    }

    pub(self) fn init(&mut self, buffer_config: &BufferConfig) {
        self.sample_rate = buffer_config.sample_rate;
        self.ts.resize(buffer_config.max_buffer_size as _, 0);
        self.output.resize(buffer_config.max_buffer_size as _, 0.0);
    }

    pub(self) fn t(&self) -> i64 {
        self.t.load(Ordering::Relaxed) as _
    }

    pub(self) fn reset_t(&self) {
        self.t.store(0.0, Ordering::Relaxed);
    }

    pub(self) fn ts(&mut self) -> *mut i64 {
        self.ts.as_mut_ptr() as _
    }

    pub(self) fn output(&mut self) -> *mut f32 {
        self.output.as_mut_ptr() as _
    }

    pub(self) fn fill_ts(&mut self) {
        let mut current_t = self.t.load(Ordering::Relaxed);
        self.ts.iter_mut().for_each(|t| {
            *t = current_t as _;
            current_t += (self.frequency.smoothed.next() / self.sample_rate) as f64;

            // NOTE: Wrap around to prevent overflow.
            if current_t > i64::MAX as f64 {
                current_t = i64::MIN as _;
            }
        });
        self.t.store(current_t, Ordering::Relaxed);
    }

    pub(self) fn copy_output(&mut self, buffer: &mut Buffer) {
        // TODO: Gain slider.
        // NOTE: Multiply by 0.5 because byte beats are fucking LOUD.
        self.output.iter_mut().for_each(|sample| {
            *sample *= 0.5;
        });

        for i in 0..buffer.channels() {
            buffer.as_slice()[i].copy_from_slice(self.output.as_slice());
        }
    }
}
