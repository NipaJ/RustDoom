use sdl2;
use sdl2::SdlResult;

pub struct Framebuffer<'a> {
	window : sdl2::video::Window,
	surface : sdl2::surface::Surface<'a>,
}

impl<'a> Framebuffer<'a> {
	pub fn new(video_ctx : &sdl2::VideoSubsystem, width : u32, height : u32) -> SdlResult<Framebuffer<'a>> {
		let mut window = try!(video_ctx.window("Doom", width, height).position_centered().build());
		window.show();

		Ok(Framebuffer {
			window: window,
			surface: try!(sdl2::surface::Surface::new(width, height, sdl2::pixels::PixelFormatEnum::BGRX8888)),
		})
	}

	pub fn get(&mut self) -> (&mut [u8], usize, usize, usize) {
		let width = self.surface.width() as usize;
		let height = self.surface.height() as usize;
		let pitch = self.surface.pitch() as usize;

		// If this panics, fix the surface creation code.
		let buffer = self.surface.without_lock_mut().unwrap();

		(buffer, width, height, pitch)
	}

	pub fn clear(&mut self, r : u8, g : u8, b : u8) {
		let (screen, width, height, pitch) = self.get();

		for y in 0..height {
			for x in 0..width {
				screen[y * pitch + x * 4 + 0] = 0x00u8;
				screen[y * pitch + x * 4 + 1] = r;
				screen[y * pitch + x * 4 + 2] = g;
				screen[y * pitch + x * 4 + 3] = b;
			}
		}
	}

	pub fn present(&self, event_pump : &sdl2::EventPump) -> bool {
		let surface = match self.window.surface(event_pump) {
			Ok(value) => value,
			Err(_) => return false
		};

		// Stupid SDL wrapper isn't implementing AsMut properly, so we need to do
		// unsafe hack.
		unsafe {
			let _ = self.surface.blit(None, sdl2::surface::Surface::from_ll(surface.raw()), None);
		}

		self.window.update_surface().unwrap();
		true
	}
}
