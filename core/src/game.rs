use anyhow::{anyhow, Result};
use async_trait::async_trait;
use web_sys::HtmlImageElement;

use self::red_hat_boy_states::*;
use crate::{
    browser,
    engine::{self, Cell, Game, Image, KeyState, Point, Rect, Renderer, Sheet},
};

const HEIGHT: i16 = 600;
const WIDTH: i16 = 1200;
const LOW_PLATFORM: i16 = 420;
const HIGH_PLATFORM: i16 = 375;

pub struct RedHatBoy {
    state_machine: RedHatBoyStateMachine,
    sprite_sheet: Sheet,
    image: HtmlImageElement,
}

impl RedHatBoy {
    fn new(sprite_sheet: Sheet, image: HtmlImageElement) -> Self {
        RedHatBoy {
            state_machine: RedHatBoyStateMachine::Idle(RedHatBoyState::new()),
            sprite_sheet,
            image,
        }
    }

    fn run_right(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Run);
    }

    fn slide(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Slide);
    }

    fn jump(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Jump);
    }

    fn update(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Update);
    }

    fn knock_out(&mut self) {
        self.state_machine = self.state_machine.transition(Event::KnockOut);
    }

    fn land_on(&mut self, position: i16) {
        self.state_machine = self.state_machine.transition(Event::Land(position));
    }

    fn pos_y(&self) -> i16 {
        self.state_machine.context().position.y
    }

    fn velocity_y(&self) -> i16 {
        self.state_machine.context().velocity.y
    }

    fn walking_speed(&self) -> i16 {
        self.state_machine.context().velocity.x
    }

    fn frame_name(&self) -> String {
        format!(
            "{} ({}).png",
            self.state_machine.frame_name(),
            (self.state_machine.context().frame / 3) + 1
        )
    }

    fn current_sprite(&self) -> Option<&Cell> {
        self.sprite_sheet.frames.get(&self.frame_name())
    }

    fn bounding_box(&self) -> Rect {
        const X_OFFSET: i16 = 56;
        const Y_OFFSET: i16 = 14;
        const WIDTH_OFFSET: i16 = 82;
        let destination_box = self.destination_box();

        Rect::new_from_x_y(
            destination_box.x() + X_OFFSET,
            destination_box.y() + Y_OFFSET,
            destination_box.width - WIDTH_OFFSET,
            destination_box.height - Y_OFFSET,
        )
    }

    fn destination_box(&self) -> Rect {
        let sprite = self.current_sprite().expect("Cell not found");

        Rect::new_from_x_y(
            self.state_machine.context().position.x + sprite.sprite_source_size.x,
            self.state_machine.context().position.y + sprite.sprite_source_size.y,
            sprite.frame.w,
            sprite.frame.h,
        )
    }

    fn draw(&self, renderer: &Renderer) {
        let sprite = self.current_sprite().expect("Cell not found");

        renderer.draw_image(
            &self.image,
            &Rect::new_from_x_y(
                sprite.frame.x,
                sprite.frame.y,
                sprite.frame.w,
                sprite.frame.h,
            ),
            &self.destination_box(),
        );
        renderer.draw_bounding_box(&self.bounding_box());
    }
}

pub enum Event {
    Run,
    Slide,
    Jump,
    Update,
    KnockOut,
    Land(i16),
}

#[derive(Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Sliding(RedHatBoyState<Sliding>),
    Jumping(RedHatBoyState<Jumping>),
    Falling(RedHatBoyState<Falling>),
    KnockedOut(RedHatBoyState<KnockedOut>),
}

impl RedHatBoyStateMachine {
    fn transition(self, event: Event) -> Self {
        match (self, event) {
            (RedHatBoyStateMachine::Idle(state), Event::Run) => state.run().into(),
            (RedHatBoyStateMachine::Idle(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Running(state), Event::Slide) => state.slide().into(),
            (RedHatBoyStateMachine::Running(state), Event::Jump) => state.jump().into(),
            (RedHatBoyStateMachine::Running(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Sliding(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Jumping(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Running(state), Event::KnockOut) => state.knock_out().into(),
            (RedHatBoyStateMachine::Jumping(state), Event::KnockOut) => state.knock_out().into(),
            (RedHatBoyStateMachine::Sliding(state), Event::KnockOut) => state.knock_out().into(),
            (RedHatBoyStateMachine::Falling(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Running(state), Event::Land(position)) => {
                state.land_on(position).into()
            }
            (RedHatBoyStateMachine::Jumping(state), Event::Land(position)) => {
                state.land_on(position).into()
            }
            (RedHatBoyStateMachine::Sliding(state), Event::Land(position)) => {
                state.land_on(position).into()
            }
            _ => self,
        }
    }

    fn frame_name(&self) -> &str {
        match self {
            RedHatBoyStateMachine::Idle(state) => state.frame_name(),
            RedHatBoyStateMachine::Running(state) => state.frame_name(),
            RedHatBoyStateMachine::Jumping(state) => state.frame_name(),
            RedHatBoyStateMachine::Sliding(state) => state.frame_name(),
            RedHatBoyStateMachine::Falling(state) => state.frame_name(),
            RedHatBoyStateMachine::KnockedOut(state) => state.frame_name(),
        }
    }

    fn context(&self) -> &RedHatBoyContext {
        match self {
            RedHatBoyStateMachine::Idle(state) => state.context(),
            RedHatBoyStateMachine::Running(state) => state.context(),
            RedHatBoyStateMachine::Jumping(state) => state.context(),
            RedHatBoyStateMachine::Sliding(state) => state.context(),
            RedHatBoyStateMachine::Falling(state) => state.context(),
            RedHatBoyStateMachine::KnockedOut(state) => state.context(),
        }
    }
}

impl From<RedHatBoyState<Idle>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Idle>) -> Self {
        RedHatBoyStateMachine::Idle(state)
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

impl From<RedHatBoyState<Jumping>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Jumping>) -> Self {
        RedHatBoyStateMachine::Jumping(state)
    }
}

impl From<SlidingEndState> for RedHatBoyStateMachine {
    fn from(state: SlidingEndState) -> Self {
        match state {
            SlidingEndState::Sliding(sliding) => sliding.into(),
            SlidingEndState::Running(running) => running.into(),
        }
    }
}

impl From<JumpingEndState> for RedHatBoyStateMachine {
    fn from(state: JumpingEndState) -> Self {
        match state {
            JumpingEndState::Jumping(jumping) => jumping.into(),
            JumpingEndState::Landing(landing) => landing.into(),
        }
    }
}

impl From<RedHatBoyState<Falling>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Falling>) -> Self {
        RedHatBoyStateMachine::Falling(state.into())
    }
}

impl From<RedHatBoyState<KnockedOut>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<KnockedOut>) -> Self {
        RedHatBoyStateMachine::KnockedOut(state.into())
    }
}

impl From<FallingEndState> for RedHatBoyStateMachine {
    fn from(state: FallingEndState) -> Self {
        match state {
            FallingEndState::Falling(falling) => falling.into(),
            FallingEndState::KnockedOut(knocked_out) => knocked_out.into(),
        }
    }
}

mod red_hat_boy_states {
    use super::HEIGHT;
    use crate::engine::Point;

    const FLOOR: i16 = 479;
    const PLAYER_HEIGHT: i16 = HEIGHT - FLOOR;
    const STARTING_POINT: i16 = -20;
    const IDLE_FRAMES: u8 = 29;
    const RUNNING_FRAMES: u8 = 23;
    const JUMPING_FRAMES: u8 = 35;
    const SLIDING_FRAMES: u8 = 14;
    const FALLING_FRAMES: u8 = 29;
    const RUNNING_SPEED: i16 = 4;
    const IDLE_FRAME_NAME: &str = "Idle";
    const RUN_FRAME_NAME: &str = "Run";
    const SLIDING_FRAME_NAME: &str = "Slide";
    const JUMPING_FRAME_NAME: &str = "Jump";
    const FALLING_FRAME_NAME: &str = "Dead";
    const JUMP_SPEED: i16 = -25;
    const GRAVITY: i16 = 1;
    const TERMINAL_VELOCITY: i16 = 20;

    #[derive(Copy, Clone)]
    pub struct RedHatBoyState<S> {
        context: RedHatBoyContext,
        _state: S,
    }

    impl<S> RedHatBoyState<S> {
        pub fn context(&self) -> &RedHatBoyContext {
            &self.context
        }

        fn update_context(&mut self, frames: u8) {
            self.context = self.context.update(frames);
        }
    }

    #[derive(Copy, Clone)]
    pub struct Idle;

    impl RedHatBoyState<Idle> {
        pub fn new() -> Self {
            RedHatBoyState {
                context: RedHatBoyContext {
                    frame: 0,
                    position: Point {
                        x: STARTING_POINT,
                        y: FLOOR,
                    },
                    velocity: Point { x: 0, y: 0 },
                },
                _state: Idle {},
            }
        }

        pub fn frame_name(&self) -> &str {
            IDLE_FRAME_NAME
        }

        pub fn run(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame().run_right(),
                _state: Running {},
            }
        }

        pub fn update(mut self) -> RedHatBoyState<Idle> {
            self.update_context(IDLE_FRAMES);
            self
        }
    }

    #[derive(Copy, Clone)]
    pub struct Running;

    impl RedHatBoyState<Running> {
        pub fn frame_name(&self) -> &str {
            RUN_FRAME_NAME
        }

        pub fn update(mut self) -> RedHatBoyState<Running> {
            self.update_context(RUNNING_FRAMES);
            self
        }

        pub fn slide(&self) -> RedHatBoyState<Sliding> {
            RedHatBoyState {
                context: self.context.reset_frame(),
                _state: Sliding {},
            }
        }

        pub fn jump(&self) -> RedHatBoyState<Jumping> {
            RedHatBoyState {
                context: self.context.reset_frame().set_vertical_velocity(JUMP_SPEED),
                _state: Jumping {},
            }
        }

        pub fn knock_out(self) -> RedHatBoyState<Falling> {
            RedHatBoyState {
                context: self.context.reset_frame().stop(),
                _state: Falling {},
            }
        }

        pub fn land_on(self, position: i16) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.set_on(position),
                _state: Running,
            }
        }
    }

    #[derive(Copy, Clone)]
    pub struct Sliding;

    pub enum SlidingEndState {
        Sliding(RedHatBoyState<Sliding>),
        Running(RedHatBoyState<Running>),
    }

    impl RedHatBoyState<Sliding> {
        pub fn frame_name(&self) -> &str {
            SLIDING_FRAME_NAME
        }

        pub fn update(mut self) -> SlidingEndState {
            self.update_context(SLIDING_FRAMES);

            if self.context.frame >= SLIDING_FRAMES {
                SlidingEndState::Running(self.stand())
            } else {
                SlidingEndState::Sliding(self)
            }
        }

        pub fn stand(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame(),
                _state: Running {},
            }
        }

        pub fn knock_out(self) -> RedHatBoyState<Falling> {
            RedHatBoyState {
                context: self.context.reset_frame().stop(),
                _state: Falling {},
            }
        }

        pub fn land_on(self, position: i16) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.set_on(position),
                _state: Running,
            }
        }
    }

    #[derive(Copy, Clone)]
    pub struct Jumping;

    pub enum JumpingEndState {
        Jumping(RedHatBoyState<Jumping>),
        Landing(RedHatBoyState<Running>),
    }

    impl RedHatBoyState<Jumping> {
        pub fn frame_name(&self) -> &str {
            JUMPING_FRAME_NAME
        }

        pub fn update(mut self) -> JumpingEndState {
            self.update_context(JUMPING_FRAMES);

            if self.context.position.y >= FLOOR {
                JumpingEndState::Landing(self.land_on(HEIGHT.into()))
            } else {
                JumpingEndState::Jumping(self)
            }
        }

        pub fn land_on(self, position: i16) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame().set_on(position),
                _state: Running {},
            }
        }

        pub fn knock_out(self) -> RedHatBoyState<Falling> {
            RedHatBoyState {
                context: self.context.reset_frame().stop(),
                _state: Falling {},
            }
        }
    }

    #[derive(Copy, Clone)]
    pub struct Falling;

    impl RedHatBoyState<Falling> {
        pub fn frame_name(&self) -> &str {
            FALLING_FRAME_NAME
        }

        pub fn update(mut self) -> FallingEndState {
            self.update_context(FALLING_FRAMES);

            if self.context.frame >= FALLING_FRAMES {
                FallingEndState::KnockedOut(self.knock_out())
            } else {
                FallingEndState::Falling(self)
            }
        }

        pub fn knock_out(self) -> RedHatBoyState<KnockedOut> {
            RedHatBoyState {
                context: self.context,
                _state: KnockedOut {},
            }
        }
    }

    pub enum FallingEndState {
        Falling(RedHatBoyState<Falling>),
        KnockedOut(RedHatBoyState<KnockedOut>),
    }

    #[derive(Copy, Clone)]
    pub struct KnockedOut;

    impl RedHatBoyState<KnockedOut> {
        pub fn frame_name(&self) -> &str {
            FALLING_FRAME_NAME
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
            if self.velocity.y < TERMINAL_VELOCITY {
                self.velocity.y += GRAVITY;
            }

            if self.frame < frame_count {
                self.frame += 1;
            } else {
                self.frame = 0;
            }

            self.position.x += self.velocity.x;
            self.position.y += self.velocity.y;

            if self.position.y > FLOOR {
                self.position.y = FLOOR;
            }

            self
        }

        fn reset_frame(mut self) -> Self {
            self.frame = 0;
            self
        }

        fn set_vertical_velocity(mut self, y: i16) -> Self {
            self.velocity.y = y;
            self
        }

        fn run_right(mut self) -> Self {
            self.velocity.x += RUNNING_SPEED;
            self
        }

        fn stop(mut self) -> Self {
            self.velocity.x = 0;
            self.velocity.y = 0;
            self
        }

        fn set_on(mut self, position: i16) -> Self {
            self.position.y = position - PLAYER_HEIGHT;
            self
        }
    }
}

pub struct Walk {
    boy: RedHatBoy,
    background: Image,
    stone: Image,
    platform: Platform,
}

pub enum WalkTheDog {
    Loading,
    Loaded(Walk),
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog::Loading {}
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        match self {
            WalkTheDog::Loading => {
                let sheet: Sheet = serde_wasm_bindgen::from_value(
                    browser::fetch_json("assets/sprite_sheets/rhb.json").await?,
                )
                .expect("rhb.json seed require");

                let background =
                    engine::load_image("assets/resized/freetileset/png/BG/BG.png").await?;

                let stone =
                    engine::load_image("assets/resized/freetileset/png/Object/Stone.png").await?;

                let rhb = RedHatBoy::new(
                    sheet,
                    engine::load_image("assets/sprite_sheets/rhb.png").await?,
                );

                let platform_sheet = serde_wasm_bindgen::from_value(
                    browser::fetch_json("assets/sprite_sheets/tiles.json").await?,
                )
                .expect("tiles.json does not exist");
                let platform = Platform::new(
                    platform_sheet,
                    engine::load_image("assets/sprite_sheets/tiles.png").await?,
                    Point {
                        x: 370,
                        y: HIGH_PLATFORM,
                    },
                );

                Ok(Box::new(WalkTheDog::Loaded(Walk {
                    boy: rhb,
                    background: Image::new(background, Point { x: 0, y: 0 }),
                    stone: Image::new(stone, Point { x: 150, y: 546 }),
                    platform,
                })))
            }
            WalkTheDog::Loaded(_) => Err(anyhow!("Error: Game is already initialized")),
        }
    }

    fn update(&mut self, keystate: &KeyState) {
        if let WalkTheDog::Loaded(walk) = self {
            if keystate.is_pressed("ArrowRight") {
                walk.boy.run_right();
            }

            if keystate.is_pressed("Space") {
                walk.boy.jump();
            }

            if keystate.is_pressed("ArrowDown") {
                walk.boy.slide();
            }

            walk.boy.update();

            for bounding_box in &walk.platform.bounding_boxes() {
                if walk.boy.bounding_box().intersects(bounding_box) {
                    if walk.boy.velocity_y() > 0 && walk.boy.pos_y() < walk.platform.position.y {
                        walk.boy.land_on(bounding_box.position.y);
                    } else {
                        walk.boy.knock_out();
                    }
                }
            }

            if walk
                .boy
                .bounding_box()
                .intersects(walk.stone.bounding_box())
            {
                walk.boy.knock_out();
            }
        }
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&&Rect::new_from_x_y(0, 0, WIDTH, HEIGHT));

        if let WalkTheDog::Loaded(walk) = self {
            walk.background.draw(renderer);
            walk.boy.draw(renderer);
            walk.stone.draw(renderer);
            walk.platform.draw(renderer);
        }
    }
}

struct Platform {
    sheet: Sheet,
    image: HtmlImageElement,
    position: Point,
}

impl Platform {
    fn new(sheet: Sheet, image: HtmlImageElement, position: Point) -> Self {
        Platform {
            sheet,
            image,
            position,
        }
    }

    fn draw(&self, renderer: &Renderer) {
        let platform = self
            .sheet
            .frames
            .get("13.png")
            .expect("13.png does not exist");

        renderer.draw_image(
            &self.image,
            &Rect::new_from_x_y(
                platform.frame.x.into(),
                platform.frame.y.into(),
                (platform.frame.w * 3).into(),
                platform.frame.h.into(),
            ),
            &&self.destination_box(),
        );

        for x in self.bounding_boxes() {
            renderer.draw_bounding_box(&x);
        }
    }

    fn destination_box(&self) -> Rect {
        let platform = self
            .sheet
            .frames
            .get("13.png")
            .expect("13.png does not exist");

        Rect::new_from_x_y(
            self.position.x.into(),
            self.position.y.into(),
            (platform.frame.w * 3).into(),
            platform.frame.h.into(),
        )
    }

    fn bounding_boxes(&self) -> Vec<Rect> {
        const X_OFFSET: i16 = 60;
        const END_HEIGHT: i16 = 54;
        let destination_box = self.destination_box();

        let bounding_box_one = Rect::new_from_x_y(
            destination_box.position.x,
            destination_box.position.y,
            X_OFFSET,
            END_HEIGHT,
        );
        let bounding_box_two = Rect::new_from_x_y(
            destination_box.position.x + X_OFFSET,
            destination_box.position.y,
            destination_box.width - (X_OFFSET * 2),
            destination_box.height,
        );
        let bounding_box_three = Rect::new_from_x_y(
            destination_box.position.x + destination_box.width - X_OFFSET,
            destination_box.position.y,
            X_OFFSET,
            END_HEIGHT,
        );

        vec![bounding_box_one, bounding_box_two, bounding_box_three]
    }
}
