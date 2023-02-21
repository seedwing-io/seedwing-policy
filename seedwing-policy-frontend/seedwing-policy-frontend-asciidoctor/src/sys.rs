use wasm_bindgen::prelude::*;

#[cfg_attr(debug_assertions, wasm_bindgen(module = "/js/debug/asciidoctor.js"))]
#[cfg_attr(
    not(debug_assertions),
    wasm_bindgen(module = "/js/release/asciidoctor.js")
)]
extern "C" {
    pub type Asciidoctor;

    #[wasm_bindgen(static_method_of=Asciidoctor)]
    pub fn convert(content: &str, options: JsValue) -> String;
}
