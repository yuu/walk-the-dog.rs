use anyhow::anyhow;
use anyhow::Result;
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::channel::oneshot::channel;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;
use web_sys::HtmlImageElement;

use crate::browser;
use crate::browser::LoopClosure;

const FRAME_SIZE: f32 = 1.0 / 60.0 * 1000.0;

type SharedLoopClosure = Rc<RefCell<Option<LoopClosure>>>;

enum KeyPress {
    KeyUp(web_sys::KeyboardEvent),
    KeyDown(web_sys::KeyboardEvent),
}

macro_rules! log {
    ( $($t:tt)* ) => {
        web_sys::console::log_1(&format!( $($t)* ).into());
    }
}

#[derive(Deserialize)]
pub struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Deserialize, Clone)]
struct SheetRect {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

#[derive(Deserialize, Clone)]
struct Cell {
    frame: SheetRect,
}

#[derive(Deserialize, Clone)]
pub struct Sheet {
    frames: HashMap<String, Cell>,
}

#[async_trait(?Send)]
pub trait Game {
    async fn initialize(&self) -> Result<Box<dyn Game>>;
    fn update(&mut self, keystate: &KeyState);
    fn draw(&self, renderer: &Renderer);
}

pub struct WalkTheDog {
    rhb: Option<RedHatBoy>,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog { rhb: None }
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        let sheet: Sheet = serde_wasm_bindgen::from_value(
            browser::fetch_json("assets/sprite_sheets/rhb.json").await?,
        )
        .expect("rhb.json seed require");

        let image = load_image("assets/sprite_sheets/rhb.png").await?;

        Ok(Box::new(WalkTheDog {
            rhb: Some(RedHatBoy::new(sheet, image)),
        }))
    }

    fn update(&mut self, keystate: &KeyState) {
        if keystate.is_pressed("ArrowDown") {
            self.rhb.as_mut().unwrap().slide();
        }
        if keystate.is_pressed("ArrowUp") {}
        if keystate.is_pressed("ArrowRight") {
            self.rhb.as_mut().unwrap().run_right();
        }
        if keystate.is_pressed("ArrowLeft") {}

        self.rhb.as_mut().unwrap().update();
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 600.0,
        });

        self.rhb.as_ref().unwrap().draw(renderer);
    }
}

pub struct GameLoop {
    last_frame: f64,
    accumulated_delta: f32,
}

impl GameLoop {
    pub async fn start(game: impl Game + 'static) -> Result<()> {
        let mut keyevent_rx = prepare_input()?;
        let mut game = game.initialize().await?;

        let mut game_loop = GameLoop {
            last_frame: browser::now()?,
            accumulated_delta: 0.0,
        };

        let renderer = Renderer {
            context: browser::context()?,
        };

        let f: SharedLoopClosure = Rc::new(RefCell::new(None));
        let g = f.clone();

        let mut keystate = KeyState::new();

        *g.borrow_mut() = Some(browser::create_ref_closure(move |perf: f64| {
            process_input(&mut keystate, &mut keyevent_rx);
            game_loop.accumulated_delta += (perf - game_loop.last_frame) as f32;
            while game_loop.accumulated_delta > FRAME_SIZE {
                game.update(&keystate);
                game_loop.accumulated_delta -= FRAME_SIZE;
            }
            game_loop.last_frame = perf;

            game.draw(&renderer);

            let _ = browser::request_animation_frame(f.borrow().as_ref().unwrap());
        }));

        browser::request_animation_frame(
            g.borrow()
                .as_ref()
                .ok_or_else(|| anyhow!("GameLoop: Loop is None"))?,
        )?;
        Ok(())
    }
}

pub struct Renderer {
    context: CanvasRenderingContext2d,
}

impl Renderer {
    pub fn clear(&self, rect: &Rect) {
        self.context
            .clear_rect(rect.x.into(), rect.y.into(), rect.w.into(), rect.h.into());
    }

    pub fn draw_image(&self, image: &HtmlImageElement, frame: &Rect, destination: &Rect) {
        self.context
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &image,
                frame.x.into(),
                frame.y.into(),
                frame.w.into(),
                frame.h.into(),
                destination.x.into(),
                destination.y.into(),
                destination.w.into(),
                destination.h.into(),
            )
            .expect("Drawing is throwing exceptions! Unrecoverable error.");
    }
}

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

pub struct KeyState {
    pressed_keys: HashMap<String, web_sys::KeyboardEvent>,
}

impl KeyState {
    fn new() -> Self {
        KeyState {
            pressed_keys: HashMap::new(),
        }
    }

    pub fn is_pressed(&self, code: &str) -> bool {
        self.pressed_keys.contains_key(code)
    }

    pub fn set_pressed(&mut self, code: &str, ev: web_sys::KeyboardEvent) {
        self.pressed_keys.insert(code.into(), ev);
    }

    pub fn set_released(&mut self, code: &str) {
        self.pressed_keys.remove(code.into());
    }
}

type KeyEventChannel = (
    mpsc::UnboundedSender<KeyPress>,
    mpsc::UnboundedReceiver<KeyPress>,
);

fn prepare_input() -> Result<mpsc::UnboundedReceiver<KeyPress>> {
    let (tx, rx): KeyEventChannel = mpsc::unbounded();
    let keydown_tx = Rc::new(RefCell::new(tx));
    let keyup_tx = Rc::clone(&keydown_tx);
    let on_keydown = browser::closure_wrap(Box::new(move |keycode: web_sys::KeyboardEvent| {
        let _ = keydown_tx
            .borrow_mut()
            .start_send(KeyPress::KeyDown(keycode));
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
    let on_keyup = browser::closure_wrap(Box::new(move |keycode: web_sys::KeyboardEvent| {
        let _ = keyup_tx.borrow_mut().start_send(KeyPress::KeyUp(keycode));
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    browser::window()?.set_onkeydown(Some(on_keydown.as_ref().unchecked_ref()));
    browser::window()?.set_onkeyup(Some(on_keyup.as_ref().unchecked_ref()));
    on_keydown.forget();
    on_keyup.forget();

    Ok(rx)
}

fn process_input(state: &mut KeyState, keyevent_rx: &mut mpsc::UnboundedReceiver<KeyPress>) {
    loop {
        match keyevent_rx.try_next() {
            Ok(None) => break,
            Err(_err) => break,
            Ok(Some(ev)) => match ev {
                KeyPress::KeyUp(ev) => state.set_released(&ev.code()),
                KeyPress::KeyDown(ev) => state.set_pressed(&ev.code(), ev),
            },
        }
    }
}

use self::read_hat_boy_states::*;

pub enum Event {
    Run,
    Slide,
}

#[derive(Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Sliding(RedHatBoyState<Sliding>),
}

impl RedHatBoyStateMachine {
    fn transition(self, event: Event) -> Self {
        match (self, event) {
            (RedHatBoyStateMachine::Idle(state), Event::Run) => state.run().into(),
            (RedHatBoyStateMachine::Running(state), Event::Slide) => state.slide().into(),
            _ => self,
        }
    }

    fn frame_name(&self) -> &str {
        match self {
            RedHatBoyStateMachine::Idle(state) => state.frame_name(),
            RedHatBoyStateMachine::Running(state) => state.frame_name(),
            RedHatBoyStateMachine::Sliding(state) => state.frame_name(),
        }
    }

    fn context(&self) -> &RedHatBoyContext {
        match self {
            RedHatBoyStateMachine::Idle(state) => &state.context(),
            RedHatBoyStateMachine::Running(state) => &state.context(),
            RedHatBoyStateMachine::Sliding(state) => state.context(),
        }
    }

    fn update(self) -> Self {
        match self {
            RedHatBoyStateMachine::Idle(mut state) => {
                state.update();
                RedHatBoyStateMachine::Idle(state)
            }
            RedHatBoyStateMachine::Running(mut state) => {
                state.update();
                RedHatBoyStateMachine::Running(state)
            }
            RedHatBoyStateMachine::Sliding(mut state) => {
                state.update();
                RedHatBoyStateMachine::Sliding(state)
            }
        }
    }
}

struct RedHatBoy {
    state_machine: RedHatBoyStateMachine,
    sprite_sheet: Sheet,
    image: HtmlImageElement,
}

impl RedHatBoy {
    fn new(sheet: Sheet, image: HtmlImageElement) -> Self {
        RedHatBoy {
            state_machine: RedHatBoyStateMachine::Idle(RedHatBoyState::new()),
            sprite_sheet: sheet,
            image,
        }
    }

    fn update(&mut self) {
        self.state_machine = self.state_machine.update();
    }

    fn draw(&self, renderer: &Renderer) {
        let frame_name = format!(
            "{} ({}).png",
            self.state_machine.frame_name(),
            (self.state_machine.context().frame / 3) + 1
        );

        let sprite = self
            .sprite_sheet
            .frames
            .get(&frame_name)
            .expect("Cell not found");

        renderer.draw_image(
            &self.image,
            &Rect {
                x: sprite.frame.x.into(),
                y: sprite.frame.y.into(),
                w: sprite.frame.w.into(),
                h: sprite.frame.h.into(),
            },
            &Rect {
                x: self.state_machine.context().position.x.into(),
                y: self.state_machine.context().position.y.into(),
                w: sprite.frame.w.into(),
                h: sprite.frame.h.into(),
            },
        );
    }

    fn run_right(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Run);
    }

    fn slide(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Slide);
    }
}

mod read_hat_boy_states {
    use crate::engine::Point;
    const FLOOR: i16 = 475;

    const IDLE_FRAMES: u8 = 29;
    const RUNNING_FRAMES: u8 = 23;
    const SLIDING_FRAMES: u8 = 14;

    use super::RedHatBoyStateMachine;

    #[derive(Copy, Clone)]
    pub struct RedHatBoyState<S> {
        context: RedHatBoyContext,
        _state: S,
    }

    impl<S> RedHatBoyState<S> {
        pub fn context(&self) -> &RedHatBoyContext {
            &self.context
        }
    }

    #[derive(Copy, Clone)]
    pub struct RedHatBoyContext {
        pub frame: u8,
        pub position: Point,
        pub velocity: Point,
    }

    impl RedHatBoyContext {
        pub fn update(mut self, frame_count: u8) -> Self {
            if self.frame < frame_count {
                self.frame += 1;
            } else {
                self.frame = 0;
            }

            self.position.x += self.velocity.x;
            self.position.y += self.velocity.y;

            self
        }

        fn reset_frame(mut self) -> Self {
            self.frame = 0;
            self
        }

        pub fn run_right(mut self) -> Self {
            self.velocity.x += 3;
            self
        }
    }

    #[derive(Copy, Clone)]
    pub struct Idle;

    #[derive(Copy, Clone)]
    pub struct Running;

    #[derive(Copy, Clone)]
    pub struct Sliding;

    impl RedHatBoyState<Idle> {
        pub fn new() -> Self {
            RedHatBoyState {
                context: RedHatBoyContext {
                    frame: 0,
                    position: Point { x: 0, y: FLOOR },
                    velocity: Point { x: 0, y: 0 },
                },
                _state: Idle {},
            }
        }

        pub fn run(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame().run_right(),
                _state: Running {},
            }
        }

        pub fn frame_name(&self) -> &str {
            "Idle"
        }

        pub fn update(&mut self) {
            self.context = self.context.update(IDLE_FRAMES);
        }
    }

    impl RedHatBoyState<Running> {
        pub fn frame_name(&self) -> &str {
            "Run"
        }

        pub fn update(&mut self) {
            self.context = self.context.update(RUNNING_FRAMES);
        }

        pub fn slide(&self) -> RedHatBoyState<Sliding> {
            RedHatBoyState {
                context: self.context.reset_frame(),
                _state: Sliding {},
            }
        }
    }

    impl RedHatBoyState<Sliding> {
        pub fn frame_name(&self) -> &str {
            "Slide"
        }

        pub fn update(&mut self) {
            self.context = self.context.update(SLIDING_FRAMES);
        }
    }

    impl From<RedHatBoyState<Running>> for RedHatBoyStateMachine {
        fn from(state: RedHatBoyState<Running>) -> Self {
            RedHatBoyStateMachine::Running(state)
        }
    }

    impl From<RedHatBoyState<Sliding>> for RedHatBoyStateMachine {
        fn from(state: RedHatBoyState<Sliding>) -> Self {
            RedHatBoyStateMachine::Sliding(state)
        }
    }
}
