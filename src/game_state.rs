use crate::error::Result;
use crate::{atlas, controls, palette, sfx, sprite, utils};
use allegro::*;
use allegro_font::*;
use allegro_image::*;
use allegro_primitives::*;
use allegro_ttf::*;
use nalgebra::{Point2, Vector2};
use serde_derive::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::{fmt, path, sync};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Options
{
	pub fullscreen: bool,
	pub width: i32,
	pub height: i32,
	pub play_music: bool,
	pub vsync_method: i32,
	pub sfx_volume: f32,
	pub music_volume: f32,
	pub camera_speed: i32,
	pub grab_mouse: bool,
	pub ui_scale: f32,
	pub frac_scale: bool,
	pub ray_casting_steps: i32,

	pub controls: controls::Controls,
}

impl Default for Options
{
	fn default() -> Self
	{
		Self {
			fullscreen: true,
			width: 960,
			height: 720,
			play_music: true,
			vsync_method: if cfg!(target_os = "windows") { 1 } else { 2 },
			sfx_volume: 1.,
			music_volume: 1.,
			camera_speed: 4,
			grab_mouse: false,
			ui_scale: 1.,
			frac_scale: true,
			ray_casting_steps: 16,
			controls: controls::Controls::new(),
		}
	}
}

#[derive(Debug)]
pub enum NextScreen
{
	Game(bool),
	Menu,
	InGameMenu,
	Quit,
}

pub struct GameState
{
	pub core: Core,
	pub prim: PrimitivesAddon,
	pub image: ImageAddon,
	pub font: FontAddon,
	pub ttf: TtfAddon,
	pub tick: i64,
	pub paused: bool,

	pub sfx: sfx::Sfx,
	pub atlas: atlas::Atlas,
	pub ui_font: Option<Font>,
	pub options: Options,
	bitmaps: HashMap<String, Bitmap>,
	sprites: HashMap<String, sprite::Sprite>,
	pub controls: controls::ControlsHandler,
	pub track_mouse: bool,
	pub mouse_pos: Point2<i32>,

	pub draw_scale: f32,
	pub display_width: f32,
	pub display_height: f32,
	pub buffer1: Option<Bitmap>,
	pub buffer2: Option<Bitmap>,

	pub light_buffer: Option<Bitmap>,
	pub ray_casting_buffer_1: Option<Bitmap>,
	pub ray_casting_buffer_2: Option<Bitmap>,
	pub distance_buffer_1: Option<Bitmap>,
	pub distance_buffer_2: Option<Bitmap>,
	pub distance_buffer_fin: Option<Bitmap>,

	pub basic_shader: sync::Weak<Shader>,
	pub palette_shader: sync::Weak<Shader>,
	pub jfa_seed_shader: sync::Weak<Shader>,
	pub jfa_jump_shader: sync::Weak<Shader>,
	pub jfa_dist_shader: sync::Weak<Shader>,
	pub ray_casting_shader: sync::Weak<Shader>,

	pub palettes: palette::PaletteList,

	pub alpha: f32,
}

pub fn load_options(core: &Core) -> Result<Options>
{
	Ok(utils::load_user_data(core, "options.cfg")?.unwrap_or_default())
}

pub fn save_options(core: &Core, options: &Options) -> Result<()>
{
	utils::save_user_data(core, "options.cfg", options)
}

impl GameState
{
	pub fn new() -> Result<Self>
	{
		let core = Core::init()?;
		core.set_app_name("BladeBlade2");
		core.set_org_name("SiegeLord");

		let options = load_options(&core)?;
		let prim = PrimitivesAddon::init(&core)?;
		let image = ImageAddon::init(&core)?;
		let font = FontAddon::init(&core)?;
		let ttf = TtfAddon::init(&font)?;
		core.install_keyboard()
			.map_err(|_| "Couldn't install keyboard".to_string())?;
		core.install_mouse()
			.map_err(|_| "Couldn't install mouse".to_string())?;

		let sfx = sfx::Sfx::new(options.sfx_volume, options.music_volume, &core)?;
		//sfx.set_music_file("data/lemonade-sinus.xm");
		//sfx.play_music()?;

		let palettes = palette::PaletteList::new(&core);

		let controls = controls::ControlsHandler::new(options.controls.clone());
		Ok(Self {
			options: options,
			core: core,
			prim: prim,
			image: image,
			tick: 0,
			bitmaps: HashMap::new(),
			sprites: HashMap::new(),
			font: font,
			ttf: ttf,
			sfx: sfx,
			paused: false,
			atlas: atlas::Atlas::new(1024),
			ui_font: None,
			draw_scale: 1.,
			display_width: 0.,
			display_height: 0.,
			buffer1: None,
			buffer2: None,
			controls: controls,
			track_mouse: true,
			mouse_pos: Point2::new(0, 0),
			palette_shader: Default::default(),
			basic_shader: Default::default(),
			jfa_seed_shader: Default::default(),
			jfa_jump_shader: Default::default(),
			jfa_dist_shader: Default::default(),
			ray_casting_shader: Default::default(),
			palettes: palettes,
			light_buffer: None,
			ray_casting_buffer_1: None,
			ray_casting_buffer_2: None,
			distance_buffer_1: None,
			distance_buffer_2: None,
			distance_buffer_fin: None,
			alpha: 0.,
		})
	}

	pub fn buffer1(&self) -> &Bitmap
	{
		self.buffer1.as_ref().unwrap()
	}

	pub fn buffer2(&self) -> &Bitmap
	{
		self.buffer2.as_ref().unwrap()
	}

	pub fn buffer_width(&self) -> f32
	{
		self.buffer1().get_width() as f32
	}

	pub fn buffer_height(&self) -> f32
	{
		self.buffer1().get_height() as f32
	}

	pub fn ui_font(&self) -> &Font
	{
		self.ui_font.as_ref().unwrap()
	}

	pub fn resize_display(&mut self, display: &Display) -> Result<()>
	{
		const FIXED_BUFFER: bool = true;

		let buffer_width;
		let buffer_height;
		if FIXED_BUFFER
		{
			buffer_width = 320 * 3 / 2;
			buffer_height = 240 * 3 / 2;
		}
		else
		{
			buffer_width = display.get_width();
			buffer_height = display.get_height();
		}

		self.display_width = display.get_width() as f32;
		self.display_height = display.get_height() as f32;
		self.draw_scale = utils::min(
			(display.get_width() as f32) / (buffer_width as f32),
			(display.get_height() as f32) / (buffer_height as f32),
		);
		if !self.options.frac_scale
		{
			self.draw_scale = self.draw_scale.floor();
		}

		if self.buffer1.is_none() || !FIXED_BUFFER
		{
			self.core.set_new_bitmap_depth(16);
			self.buffer1 = Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());
			self.core.set_new_bitmap_depth(0);
			self.buffer2 = Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());

			let old_flags = self.core.get_new_bitmap_flags();
			self.core.set_new_bitmap_flags(MAG_LINEAR | MIN_LINEAR);
			self.light_buffer = Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());
			self.ray_casting_buffer_1 =
				Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());
			self.ray_casting_buffer_2 =
				Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());
			self.core.set_new_bitmap_flags(old_flags);

			let old_format = self.core.get_new_bitmap_format();
			self.core
				.set_new_bitmap_format(PixelFormat::PixelFormatAbgrF32);
			self.distance_buffer_1 =
				Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());
			self.distance_buffer_2 =
				Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());
			self.core.set_new_bitmap_format(old_format);

			self.distance_buffer_fin =
				Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());
		}

		//self.ui_font = Some(
		//	Font::new_builtin(&self.font)
		//		.map_err(|_| "Couldn't create builtin font".to_string())?,
		//);

		//self.ui_font = Some(utils::load_ttf_font(
		//	&self.ttf,
		//	"data/jupiterc.ttf",
		//	(20. * self.options.ui_scale) as i32,
		//)?);

		self.ui_font = Some(utils::load_ttf_font(
			&self.ttf,
			"data/Pixel Musketeer.ttf",
			(14. * self.options.ui_scale) as i32,
		)?);

		Ok(())
	}

	pub fn transform_mouse(&self, x: f32, y: f32) -> (f32, f32)
	{
		let x = (x - self.display_width / 2.) / self.draw_scale + self.buffer_width() / 2.;
		let y = (y - self.display_height / 2.) / self.draw_scale + self.buffer_height() / 2.;
		(x, y)
	}

	pub fn cache_bitmap<'l>(&'l mut self, name: &str) -> Result<&'l Bitmap>
	{
		Ok(match self.bitmaps.entry(name.to_string())
		{
			Entry::Occupied(o) => o.into_mut(),
			Entry::Vacant(v) => v.insert(utils::load_bitmap(&self.core, name)?),
		})
	}

	pub fn cache_sprite<'l>(&'l mut self, name: &str) -> Result<&'l sprite::Sprite>
	{
		Ok(match self.sprites.entry(name.to_string())
		{
			Entry::Occupied(o) => o.into_mut(),
			Entry::Vacant(v) => v.insert(sprite::Sprite::load(
				name,
				&self.core,
				&mut self.atlas,
				&mut self.palettes,
			)?),
		})
	}

	pub fn get_bitmap<'l>(&'l self, name: &str) -> Result<&'l Bitmap>
	{
		Ok(self
			.bitmaps
			.get(name)
			.ok_or_else(|| format!("{name} is not cached!"))?)
	}

	pub fn get_sprite<'l>(&'l self, name: &str) -> Result<&'l sprite::Sprite>
	{
		Ok(self
			.sprites
			.get(name)
			.ok_or_else(|| format!("{name} is not cached!"))?)
	}

	pub fn time(&self) -> f64
	{
		self.tick as f64 * utils::DT as f64
	}
}

pub fn light_pass(state: &GameState) -> Option<&Bitmap>
{
	state
		.core
		.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::Zero);
	// Seed distance buffer
	state
		.core
		.set_target_bitmap(state.distance_buffer_1.as_ref());
	state
		.core
		.use_shader(Some(&*state.basic_shader.upgrade().unwrap()))
		.unwrap();
	state.core.clear_to_color(Color::from_rgb_f(0.0, 0.0, 0.0));

	let buffer_size = Vector2::new(state.buffer_width() as f32, state.buffer_height() as f32);
	state
		.core
		.use_shader(Some(&*state.jfa_seed_shader.upgrade().unwrap()))
		.unwrap();
	state
		.core
		.set_shader_uniform("bitmap_size", &[[buffer_size.x, buffer_size.y]][..])
		.ok();
	state
		.core
		.draw_bitmap(state.light_buffer.as_ref().unwrap(), 0., 0., Flag::zero());

	// JFA
	let num_passes = utils::max(buffer_size.x, buffer_size.y).log2().ceil() as i32;
	let buffers = [
		state.distance_buffer_1.as_ref(),
		state.distance_buffer_2.as_ref(),
	];
	for i in 0..num_passes
	{
		let src_buffer = buffers[(i % 2) as usize];
		let dst_buffer = buffers[(1 - i % 2) as usize];
		state.core.set_target_bitmap(dst_buffer);
		state
			.core
			.use_shader(Some(&*state.jfa_jump_shader.upgrade().unwrap()))
			.unwrap();
		state
			.core
			.set_shader_uniform("bitmap_size", &[[buffer_size.x, buffer_size.y]][..])
			.ok();
		state
			.core
			.set_shader_uniform(
				"uv_offset",
				&[2.0_f32.powf((num_passes - i - 1) as f32)][..],
			)
			.ok();
		state
			.core
			.draw_bitmap(src_buffer.unwrap(), 0., 0., Flag::zero());
	}
	let src_buffer = buffers[(num_passes % 2) as usize];
	state
		.core
		.set_target_bitmap(state.distance_buffer_fin.as_ref());
	state
		.core
		.use_shader(Some(&*state.jfa_dist_shader.upgrade().unwrap()))
		.unwrap();
	state
		.core
		.draw_bitmap(src_buffer.unwrap(), 0., 0., Flag::zero());

	// Ray casting.
	let rc_buffer;
	if false
	{
		state
			.core
			.set_target_bitmap(state.ray_casting_buffer_1.as_ref());
		state
			.core
			.use_shader(Some(&*state.ray_casting_shader.upgrade().unwrap()))
			.unwrap();
		state
			.core
			.set_shader_uniform("num_rays", &[128][..])
			.unwrap();
		state
			.core
			.set_shader_uniform("num_steps", &[32][..])
			.unwrap();
		state
			.core
			.set_shader_sampler(
				"distance_map",
				state.distance_buffer_fin.as_ref().unwrap(),
				2,
			)
			.ok();
		state
			.core
			.draw_bitmap(state.light_buffer.as_ref().unwrap(), 0., 0., Flag::zero());
		rc_buffer = state.ray_casting_buffer_1.as_ref();
	}
	else
	{
		let buffers = [
			state.ray_casting_buffer_1.as_ref(),
			state.ray_casting_buffer_2.as_ref(),
		];
		let diag = buffer_size.norm();
		let base = 4.0_f32;
		let num_cascades = (diag.ln() / base.ln()).ceil() + 1.;

		let last_idx = 0;
		for i in (last_idx..=num_cascades as i32 - 1).rev()
		{
			let src_buffer = buffers[(i % 2) as usize];
			let dst_buffer = buffers[(1 - i % 2) as usize];
			state.core.set_target_bitmap(dst_buffer);
			state
				.core
				.use_shader(Some(&*state.ray_casting_shader.upgrade().unwrap()))
				.unwrap();
			state
				.core
				.set_shader_sampler(
					"distance_map",
					state.distance_buffer_fin.as_ref().unwrap(),
					2,
				)
				.ok();
			state
				.core
				.set_shader_sampler("prev_cascade", src_buffer.unwrap(), 3)
				.ok();
			state.core.set_shader_uniform("base", &[base][..]).ok();
			state
				.core
				.set_shader_uniform("bitmap_size", &[[buffer_size.x, buffer_size.y]][..])
				.ok();
			state
				.core
				.set_shader_uniform("cascade_index", &[i as f32][..])
				.ok();
			state
				.core
				.set_shader_uniform("num_cascades", &[num_cascades as f32][..])
				.ok();
			state
				.core
				.set_shader_uniform("last_index", &[(i == last_idx) as i32][..])
				.ok();
			state
				.core
				.set_shader_uniform("num_steps", &[state.options.ray_casting_steps][..])
				.ok();
			state
				.core
				.draw_bitmap(state.light_buffer.as_ref().unwrap(), 0., 0., Flag::zero());
		}
		rc_buffer = buffers[1 - last_idx as usize % 2];
	}

	// Debug
	// state.core.set_target_bitmap(state.buffer1.as_ref());
	// state
	//  .core
	//  .use_shader(Some(&*state.basic_shader.upgrade().unwrap()))
	//  .unwrap();
	// state.core.clear_to_color(Color::from_rgb_f(0.0, 0.0, 0.1));
	// state
	// 	.core
	// 	.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::InverseAlpha);
	// state.core.draw_bitmap(
	// 	//buffers[num_passes as usize % 2].unwrap(),
	// 	state.light_buffer.as_ref().unwrap(),
	// 	//dist_buffer.unwrap(),
	// 	//rc_buffer.unwrap(),
	// 	0.,
	// 	0.,
	// 	Flag::zero(),
	// );
	rc_buffer
}
