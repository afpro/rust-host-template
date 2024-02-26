use {crate::core::ApiCore, wasm_bindgen::prelude::wasm_bindgen};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
}

#[wasm_bindgen]
#[derive(Default)]
pub struct ApiClient {
    _core: ApiCore,
}

#[wasm_bindgen]
impl ApiClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(debug_assertions)]
    pub async fn say_hi(&self) {
        console_log("hi");
    }
}
