use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct MarkerData {
    message: String,
}

#[wasm_bindgen]
impl MarkerData {
    #[wasm_bindgen(constructor)]
    pub fn new(message: String) -> Self {
        Self { message }
    }
}
