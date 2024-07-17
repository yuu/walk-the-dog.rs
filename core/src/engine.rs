use anyhow::anyhow;
use anyhow::Result;
use futures::channel::oneshot::channel;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::HtmlImageElement;

use crate::browser;

pub async fn load_image(source: &str) -> Result<HtmlImageElement> {
    let image = browser::new_image()?;

    let (success_tx, success_rx) = channel::<Result<()>>();
    let success_tx = Rc::new(Mutex::new(Some(success_tx)));
    let error_tx = Rc::clone(&success_tx);
    let callback = browser::closure_once(move || {
        if let Some(tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
            let _ = tx.send(Ok(()));
        };
    });
    let error_callback: Closure<dyn FnMut(JsValue)> = browser::closure_once(move |err| {
        if let Some(tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
            let _ = tx.send(Err(anyhow!("Error loading Image: {:#?}", err)));
        };
    });
    image.set_onload(Some(callback.as_ref().unchecked_ref()));
    image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));
    image.set_src(source);

    let _ = success_rx.await??;

    Ok(image)
}
