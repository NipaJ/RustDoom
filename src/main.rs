extern crate sdl2;
extern crate byteorder;
pub mod system;
pub mod framebuffer;
pub mod resources;

use std::f32;
use system::System;
use system::Keycode;
use system::KeyEvent;
use framebuffer::Framebuffer;
use resources::ResourceManager;
use resources::bsp;

struct Camera {
	pos : (i32, i32),
	angle : u16,

	// Hacky movement system
	forward_movement : i32,
	side_movement : i32
}

impl Camera {
	fn new(pos : (i32, i32), angle : u16) -> Camera {
		Camera {
			pos: pos,
			angle: angle,
			forward_movement: 0i32,
			side_movement: 0i32,
		}
	}

	fn process_input(&mut self, system : &System) {
		for &event in system.key_events() {
			match event {
				KeyEvent::Down(code) => {
					match code {
						Keycode::Left => self.side_movement -= 1,
						Keycode::Right => self.side_movement += 1,
						Keycode::Down => self.forward_movement -= 1,
						Keycode::Up => self.forward_movement += 1,
						_ => ()
					}
				}
				KeyEvent::Up(code) => {
					match code {
						Keycode::Left => self.side_movement += 1,
						Keycode::Right => self.side_movement -= 1,
						Keycode::Down => self.forward_movement += 1,
						Keycode::Up => self.forward_movement -= 1,
						_ => ()
					}
				}
			}
		}

		// Turn
		if self.side_movement > 0 {
			self.angle = self.angle.wrapping_add(182u16);
		} else if self.side_movement < 0 {
			self.angle = self.angle.wrapping_sub(182u16);
		}

		// Move forward
		let angle = (self.angle as f32) / (0x10000 as f32) * f32::consts::PI * 2f32;
		let dir_x = (angle.cos() * (0x10000 as f32)) as i32;
		let dir_y = (angle.sin() * (0x10000 as f32)) as i32;
		self.pos.0 += dir_x * self.forward_movement;
		self.pos.1 += dir_y * self.forward_movement;

		println!("{:?}", (self.pos, self.angle));
	}
}

struct Renderer<'a, 'b : 'a> {
	level: &'a bsp::Map,
	fb: &'a mut Framebuffer<'b>
}

impl<'a, 'b> Renderer<'a, 'b> {
	pub fn new(level : &'a bsp::Map, framebuffer : &'a mut Framebuffer<'b>) -> Renderer<'a, 'b> {
		Renderer {
			level: level,
			fb: framebuffer
		}
	}

	pub fn render_view(&mut self, camera : &Camera) {
		self.render_bsp_node(camera, self.level.nodes.len() as u32 - 1);
	}

	fn render_bsp_node(&mut self, camera : &Camera, node : u32) {
		if node & 0x80000000 != 0 {
			self.render_subsector(camera, node & 0x7FFFFFFF);
			return;
		}

		let node = &self.level.nodes[node as usize];
		let side = get_leaf_side(node, camera.pos) as usize;

		self.render_bsp_node(camera, node.child[side]);

		// TODO: Use the bounding box to filter out nodes that are not facing the camera.
		self.render_bsp_node(camera, node.child[side ^ 1]);
	}

	fn render_subsector(&mut self, camera : &Camera, node : u32) {
		let subsector = &self.level.subsectors[node as usize];

		let segs = &self.level.segs[(subsector.first_seg as usize)..(subsector.first_seg as usize + subsector.num_segs as usize)];
		for seg in segs {
			self.draw_line(camera, seg);
		}
	}

	fn draw_line(&mut self, camera : &Camera, seg : &bsp::LineSegment) {
		let v0 = self.level.vertices[seg.v[0] as usize];
		let v1 = self.level.vertices[seg.v[1] as usize];

		let (cam_pos_x, cam_pos_y) = camera.pos;
		let (cam_dir_x, cam_dir_y) = get_direction(camera.angle);

		let tx0 = v0.x - cam_pos_x;
		let tx1 = v1.x - cam_pos_x;
		let ty0 = v0.y - cam_pos_y;
		let ty1 = v1.y - cam_pos_y;

		// Backface culling
		if ((fixed_mul(ty0, tx0 - tx1) + fixed_mul(tx0, ty1 - ty0)) >> 16) >= 0 {
			return;
		}

		let vx0 = fixed_mul(tx0, cam_dir_y) + fixed_mul(ty0, -cam_dir_x);
		let vx1 = fixed_mul(tx1, cam_dir_y) + fixed_mul(ty1, -cam_dir_x);
		let vz0 = fixed_mul(tx0, cam_dir_x) + fixed_mul(ty0, cam_dir_y);
		let vz1 = fixed_mul(tx1, cam_dir_x) + fixed_mul(ty1, cam_dir_y);
	}
}

fn get_direction(angle: u16) -> (i32, i32) {
	let fangle = (angle as f32) / (0x10000 as f32) * f32::consts::PI * 2f32;
	let dir_x = (fangle.cos() * (0x10000 as f32)) as i32;
	let dir_y = (fangle.sin() * (0x10000 as f32)) as i32;
	(dir_x, dir_y)
}

fn get_leaf_side(node : &bsp::Node, (x, y) : (i32, i32)) -> bool {
	if node.dx == 0 {
		return if x <= node.x { node.dy > 0 } else { node.dy < 0 };
	}

	if node.dy == 0 {
		return if y <= node.y { node.dx < 0 } else { node.dx > 0 };
	}

	let dx = x - node.x;
	let dy = y - node.y;

	fixed_mul(dy, node.dx >> 16) >= fixed_mul(node.dy >> 16, dx)
}

fn fixed_mul(a : i32, b : i32) -> i32 {
	((a as i64 * b as i64) >> 16i64) as i32
}

fn fixed_div(a : i32, b : i32) -> i32 {
	(((a as i64) << 16i64) / b as i64) as i32
}

fn main() {
	let mut resource_manager = ResourceManager::new();
	resource_manager.load_package("./doom1.wad").unwrap();

	let mut system = System::new().unwrap();
	let mut framebuffer = system.create_framebuffer(1024, 768).unwrap();

	let mut camera = Camera::new((0, 0), 0);

	while system.handle_events() {
		camera.process_input(&system);

		let level = if let Some(value) = resource_manager.find_map("E1M1") {
			value
		} else {
			panic!("Cannot load level E1M1");
		};

		{
			let mut renderer = Renderer::new(level, &mut framebuffer);
			renderer.render_view(&camera);
		}

		system.present(&framebuffer);
	}
}
