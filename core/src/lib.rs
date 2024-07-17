use wasm_bindgen::prelude::*;

mod browser;
mod engine;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, wasm!");
}

#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    browser::spawn_local(async move {
        let game = engine::WalkTheDog::new();

        engine::GameLoop::start(game)
            .await
            .expect("Could not start game loop");
    });

    Ok(())
}
