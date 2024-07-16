use rand::prelude::*;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    wasm_bindgen_futures::spawn_local(async move {
        let (success_tx, success_rx) = futures::channel::oneshot::channel::<Result<(), JsValue>>();
        let success_tx = Rc::new(Mutex::new(Some(success_tx)));
        let error_tx = Rc::clone(&success_tx);

        let image = web_sys::HtmlImageElement::new().unwrap();
        image.set_src("assets/resized/rhb/Idle (1).png");

        let callback = Closure::once(move || {
            if let Some(tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
                tx.send(Ok(()));
            };
        });
        let error_callback = Closure::once(move |err| {
            if let Some(tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
                tx.send(Err(err));
            };
        });

        image.set_onload(Some(callback.as_ref().unchecked_ref()));
        image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));
        callback.forget();

        success_rx.await;
        let _ = context.draw_image_with_html_image_element(&image, 0.0, 0.0);
    });

    Ok(())
}
