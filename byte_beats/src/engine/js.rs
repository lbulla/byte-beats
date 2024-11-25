use std::ffi::{CStr, CString};
use std::ptr::null_mut;
use std::sync::Arc;

use libquickjs_sys as q;

use nih_plug::audio_setup::BufferConfig;
use nih_plug::buffer::Buffer;
use nih_plug::params::FloatParam;
use nih_plug::plugin::ProcessStatus;

use super::{Engine, EngineBase};

pub(crate) struct JsEngine {
    base: EngineBase,

    rt: *mut q::JSRuntime,
    ctx: *mut q::JSContext,

    generator: q::JSValue,
    generate_func: q::JSValue,
}

impl JsEngine {
    pub(crate) fn new(frequency: Arc<FloatParam>) -> Self {
        unsafe {
            let rt = q::JS_NewRuntime();
            let ctx = q::JS_NewContext(rt);

            let eval_result = q::JS_Eval(
                ctx,
                SCRIPT.as_ptr(),
                SCRIPT.count_bytes() as _,
                FILE_NAME.as_ptr(),
                q::JS_EVAL_TYPE_GLOBAL as _,
            );
            q::JS_FreeValue(ctx, eval_result);

            let global = q::JS_GetGlobalObject(ctx);

            let create_func = q::JS_GetPropertyStr(ctx, global, c"createGenerator".as_ptr());
            let generator = q::JS_Call(ctx, create_func, global, 0, null_mut());
            let generate_func = q::JS_GetPropertyStr(ctx, generator, c"generate".as_ptr() as _);

            q::JS_FreeValue(ctx, create_func);
            q::JS_FreeValue(ctx, global);

            Self {
                base: EngineBase::new(frequency),

                rt,
                ctx,

                generator,
                generate_func,
            }
        }
    }
}

impl Engine for JsEngine {
    fn init(&mut self, buffer_config: &BufferConfig) -> bool {
        self.base.init(buffer_config);

        let max_buffer_size = buffer_config.max_buffer_size as usize;
        unsafe {
            let ts = q::JS_NewArrayBuffer(
                self.ctx,
                self.base.ts() as _,
                (max_buffer_size * size_of::<i64>()) as _,
                None,
                null_mut(),
                0,
            );

            let output = q::JS_NewArrayBuffer(
                self.ctx,
                self.base.output() as _,
                (max_buffer_size * size_of::<f32>()) as _,
                None,
                null_mut(),
                0,
            );

            let set_buffers_func =
                q::JS_GetPropertyStr(self.ctx, self.generator, c"setBuffers".as_ptr());
            let mut args = [ts, output];
            let result = q::JS_Call(
                self.ctx,
                set_buffers_func,
                self.generator,
                args.len() as _,
                args.as_mut_ptr(),
            );

            q::JS_FreeValue(self.ctx, result);
            q::JS_FreeValue(self.ctx, ts);
            q::JS_FreeValue(self.ctx, output);
            q::JS_FreeValue(self.ctx, set_buffers_func);
        }
        true
    }

    fn process(&mut self, buffer: &mut Buffer) -> ProcessStatus {
        self.base.fill_ts();

        unsafe {
            // FIXME: Why is this needed?
            q::JS_UpdateStackTop(self.rt);

            let mut num_samples = q::JS_NewInt32(self.ctx, buffer.samples() as _);
            let result = q::JS_Call(
                self.ctx,
                self.generate_func,
                self.generator,
                1,
                &raw mut num_samples,
            );
            q::JS_FreeValue(self.ctx, result);
            q::JS_FreeValue(self.ctx, num_samples);
        }

        self.base.copy_output(buffer);
        ProcessStatus::Normal
    }

    fn t(&self) -> i64 {
        self.base.t()
    }

    fn reset_t(&self) {
        self.base.reset_t()
    }

    fn set_func(&mut self, func: &str) {
        let func = format!(
            r#"
function createGenerateSample() {{
    return function generateSample(t) {{
        return {func};
    }};
}}"#
        );
        let func = CString::new(func).unwrap();

        unsafe {
            let result = q::JS_Eval(
                self.ctx,
                func.as_ptr(),
                func.count_bytes() as _,
                FILE_NAME.as_ptr() as _,
                q::JS_EVAL_TYPE_GLOBAL as _,
            );
            q::JS_FreeValue(self.ctx, result);

            let global = q::JS_GetGlobalObject(self.ctx);

            let func_creator_js =
                q::JS_GetPropertyStr(self.ctx, global, c"createGenerateSample".as_ptr());
            let mut func_js = q::JS_Call(self.ctx, func_creator_js, global, 0, null_mut());

            let set_func =
                q::JS_GetPropertyStr(self.ctx, self.generator, c"setGenerateSample".as_ptr());
            q::JS_Call(self.ctx, set_func, self.generator, 1, &raw mut func_js);

            q::JS_FreeValue(self.ctx, set_func);
            q::JS_FreeValue(self.ctx, func_js);
            q::JS_FreeValue(self.ctx, func_creator_js);
            q::JS_FreeValue(self.ctx, global);
        }
    }
}

impl Drop for JsEngine {
    fn drop(&mut self) {
        unsafe {
            q::JS_FreeValue(self.ctx, self.generate_func);
            q::JS_FreeValue(self.ctx, self.generator);

            q::JS_FreeContext(self.ctx);
            q::JS_FreeRuntime(self.rt);
        }
    }
}

unsafe impl Send for JsEngine {}
unsafe impl Sync for JsEngine {}

unsafe fn get_exception(ctx: *mut q::JSContext) -> Option<String> {
    let exception_js = q::JS_GetException(ctx);
    let mut len = 0;
    let exception_c_char = q::JS_ToCStringLen2(ctx, &raw mut len, exception_js, 0);

    let exception;
    if exception_c_char.is_null() {
        exception = None;
    } else {
        let exception_cstr = CStr::from_ptr(exception_c_char);
        exception = Some(exception_cstr.to_str().unwrap().to_owned());
    }

    q::JS_FreeValue(ctx, exception_js);
    exception
}

const SCRIPT: &'static CStr = cr#"
class Context {
    generateSample(t) {
        return t;
    }
}

class Generator {
    constructor() {
        this.context = new Context;
    }

    setBuffers(ts, output) {
        this.ts = new BigInt64Array(ts);
        this.output = new Float32Array(output);
    }

    normSample(sample) {
        return (sample & 255) / 127 - 1;
    }

    generate(numSamples) {
        for (let i = 0; i < numSamples; ++i) {
            let t = Number(this.ts[i]);
            let sample = this.context.generateSample(t);
            this.output[i] = this.normSample(sample);
        }
    }

    setGenerateSample(newFunction) {
        this.context.generateSample = newFunction;
    }
}

function createGenerator() {
    return new Generator;
}
"#;

const FILE_NAME: &'static CStr = c"<eval>";
