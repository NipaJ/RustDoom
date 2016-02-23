use std::slice::Iter;
use sdl2;
use sdl2::SdlResult;
use framebuffer::Framebuffer;

pub use sdl2::keyboard::Keycode;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum KeyEvent {
    Down(Keycode),
    Up(Keycode),
}

pub struct System {
    video_ctx : sdl2::VideoSubsystem,
    event_pump : sdl2::EventPump,
    key_events : Vec<KeyEvent>
}

impl System {
    pub fn new() -> SdlResult<System> {
        let sdl_ctx = try!(sdl2::init());
        let video_ctx = try!(sdl_ctx.video());
        let event_pump = try!(sdl_ctx.event_pump());

        Ok(System {
            video_ctx: video_ctx,
            event_pump: event_pump,
            key_events: Vec::<KeyEvent>::new()
        })
    }

    pub fn create_framebuffer<'a>(&self, width : u32, height : u32) -> SdlResult<Framebuffer<'a>> {
        return Framebuffer::new(&self.video_ctx, width, height);
    }

    pub fn handle_events(&mut self) -> bool {
        use sdl2::event::Event;

        self.key_events.clear();

        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit {..} => return false,
                Event::KeyDown { repeat: false, keycode: Some(code), .. } => self.key_events.push(KeyEvent::Down(code)),
                Event::KeyUp { repeat: false, keycode: Some(code), .. } => self.key_events.push(KeyEvent::Up(code)),
                _ => ()
            }
        }
        true
    }

    pub fn key_events(&self) -> Iter<KeyEvent> {
        self.key_events.iter()
    }

    pub fn present(&self, fb : &Framebuffer) -> bool {
        fb.present(&self.event_pump)
    }
}
