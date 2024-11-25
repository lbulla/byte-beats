use std::sync::Arc;

use nih_plug::prelude::{Editor, FloatParam};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::views::Textbox;
use nih_plug_vizia::widgets::{ParamSlider, ResizeHandle};
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};

use crate::EngineInterface;

#[derive(Clone, Lens)]
pub(crate) struct Data {
    pub(crate) frequency_param: Arc<FloatParam>,
    pub(crate) engine_interface: EngineInterface,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (300, 150))
}

pub(crate) fn create(editor_state: Arc<ViziaState>, editor_data: Data) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        editor_data.clone().build(cx);

        VStack::new(cx, |cx| {
            ParamSlider::new(cx, Data::frequency_param, |param| param.as_ref())
                .width(Stretch(1.0))
                .height(Pixels(20.0));

            let func_lens = Data::engine_interface.map(|interface| interface.func());
            Textbox::new(cx, func_lens)
                .width(Stretch(1.0))
                .height(Pixels(100.0))
                .font_size(12.0)
                .on_submit(|cx, value, success| {
                    if success {
                        let data: Option<&Data> = cx.data();
                        data.unwrap().engine_interface.set_func(value);
                    }
                });

            let t_lens =
                Data::engine_interface.map(|interface| format!("t: {}", interface.t().to_string()));
            Label::new(cx, t_lens)
                .width(Pixels(280.0))
                .height(Pixels(20.0))
                .left(Pixels(8.0))
                .on_mouse_down(|cx, button| {
                    if button == MouseButton::Left {
                        let data: Option<&Data> = cx.data();
                        data.unwrap().engine_interface.reset_t();
                    }
                });
        })
        .width(Pixels(300.0));

        ResizeHandle::new(cx);
    })
}
