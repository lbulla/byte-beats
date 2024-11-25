mod editor;
mod engine;

use parking_lot::RwLock;
use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;

use crate::engine::js::JsEngine;
use crate::engine::Engine;

const DEFAULT_FUNC: &'static str = "t";

#[derive(Clone)]
struct EngineInterface {
    engine: Arc<RwLock<dyn Engine>>,
    func_param: Arc<RwLock<String>>,
}

impl EngineInterface {
    fn func(&self) -> String {
        self.func_param.read().clone()
    }

    fn set_func(&self, func: String) {
        let mut func_param = self.func_param.write();
        *func_param = func;
        self.engine.write().set_func(func_param.as_str());
    }

    fn t(&self) -> i64 {
        self.engine.read().t()
    }

    fn reset_t(&self) {
        self.engine.read().reset_t();
    }
}

#[derive(Params)]
struct ByteBeatsParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,

    #[id = "frequency"]
    frequency: Arc<FloatParam>,

    #[persist = "func"]
    func: Arc<RwLock<String>>,
}

struct ByteBeats {
    params: Arc<ByteBeatsParams>,
    engine: Arc<RwLock<dyn Engine>>,
}

impl Plugin for ByteBeats {
    const NAME: &'static str = "Byte Beats";
    const VENDOR: &'static str = "Lukas Bulla";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.editor_state.clone(),
            editor::Data {
                frequency_param: self.params.frequency.clone(),
                engine_interface: EngineInterface {
                    engine: self.engine.clone(),
                    func_param: self.params.func.clone(),
                },
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let mut engine = self.engine.write();
        // TODO: Only set the function when it has changed (e.g. after a state has been loaded)
        engine.set_func(self.params.func.read().as_str());
        engine.init(buffer_config)
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.engine.write().process(buffer)
    }
}

impl Default for ByteBeats {
    fn default() -> Self {
        let frequency = Arc::new(
            FloatParam::new(
                "Frequency",
                8000.0,
                FloatRange::Linear {
                    min: 1000.0,
                    max: 48000.0,
                },
            )
            .with_unit(" Hz"),
        );

        Self {
            params: Arc::new(ByteBeatsParams {
                editor_state: editor::default_state(),
                frequency: frequency.clone(),
                func: Arc::new(RwLock::new(DEFAULT_FUNC.to_owned())),
            }),
            engine: Arc::new(RwLock::new(JsEngine::new(frequency))),
        }
    }
}

#[cfg(target_os = "macos")]
impl AuPlugin for ByteBeats {}

impl ClapPlugin for ByteBeats {
    const CLAP_ID: &'static str = "com.lukas-bulla.byte-beats";
    const CLAP_DESCRIPTION: Option<&'static str> = None;
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for ByteBeats {
    const VST3_CLASS_ID: [u8; 16] = *b"ByteBeatsByteBea";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

#[cfg(target_os = "macos")]
nih_export_au!(ByteBeats);
nih_export_clap!(ByteBeats);
nih_export_vst3!(ByteBeats);
