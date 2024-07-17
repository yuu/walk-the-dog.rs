use serde::Deserialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

mod browser;
mod engine;

#[derive(Deserialize)]
struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Deserialize)]
struct Cell {
    frame: Rect,
}

#[derive(Deserialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

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

    let context = browser::context().expect("Could not get browser context");

    browser::spawn_local(async move {
        let sheet: Sheet = browser::fetch_json("assets/sprite_sheets/rhb.json")
            .await
            .expect("Could not fetch rhb.json")
            .into_serde()
            .expect("Could not convert rhb.json into a Sheet structure");

        let image = engine::load_image("assets/sprite_sheets/rhb.png")
            .await
            .expect("Could not load rhb.png");

        let interval_callback = Closure::wrap(Box::new(move || {}) as Box<dyn FnMut()>);
        interval_callback.forget();
    });

    Ok(())
}
