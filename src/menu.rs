use crate::error::Result;
use crate::{components, controls, game, game_state, ui, utils};

use allegro::*;
use allegro_font::*;
use allegro_sys::*;
use nalgebra::{Matrix4, Point2};
use rand::prelude::*;

pub struct Menu
{
	subscreens: ui::SubScreens,
	start_time: f64,
}

fn to_f32(pos: Point2<i32>) -> Point2<f32>
{
	Point2::new(pos.x as f32, pos.y as f32)
}

impl Menu
{
	pub fn new(state: &mut game_state::GameState) -> Result<Self>
	{
		state.paused = false;
		state.sfx.cache_sample("data/ui1.ogg")?;
		state.sfx.cache_sample("data/ui2.ogg")?;
		state.cache_sprite("data/logo.cfg")?;
		state.sfx.set_music_file("data/title.ogg", 1.);
		state.sfx.play_music()?;

		let subscreens = ui::SubScreens::new(state);

		Ok(Self {
			subscreens: subscreens,
			start_time: state.time(),
		})
	}

	pub fn input(
		&mut self, event: &Event, state: &mut game_state::GameState,
	) -> Result<Option<game_state::NextScreen>>
	{
		match *event
		{
			Event::MouseAxes { x, y, .. } =>
			{
				if state.track_mouse
				{
					let (x, y) = state.transform_mouse(x as f32, y as f32);
					state.mouse_pos = Point2::new(x as i32, y as i32);
				}
			}
			Event::KeyDown { .. } | Event::JoystickButtonDown { .. } =>
			{
				if self.subscreens.is_empty()
				{
					self.subscreens
						.push(ui::SubScreen::MainMenu(ui::MainMenu::new(state)?));
					self.subscreens.time_to_transition = state.time();
					return Ok(None);
				}
			}
			_ => (),
		}
		if !self.subscreens.is_empty()
		{
			if let Some(action) = self.subscreens.input(state, event)?
			{
				match action
				{
					ui::Action::Start => return Ok(Some(game_state::NextScreen::Game(false))),
					ui::Action::Resume => return Ok(Some(game_state::NextScreen::Game(true))),
					ui::Action::Quit => return Ok(Some(game_state::NextScreen::Quit)),
					_ => (),
				}
			}
		}
		Ok(None)
	}

	pub fn draw(&mut self, state: &game_state::GameState) -> Result<()>
	{
		state.core.set_target_bitmap(state.light_buffer.as_ref());
		let ortho_mat = Matrix4::new_orthographic(
			0.,
			state.buffer_width() as f32,
			state.buffer_height() as f32,
			0.,
			state.buffer_height(),
			-state.buffer_height(),
		);
		state
			.core
			.use_projection_transform(&utils::mat4_to_transform(ortho_mat));
		state
			.core
			.use_shader(Some(&*state.basic_shader.upgrade().unwrap()))
			.unwrap();
		state
			.core
			.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::Zero);
		state
			.core
			.clear_to_color(Color::from_rgba_f(0.0, 0.0, 0.0, 0.));
		state
			.core
			.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::InverseAlpha);

		let center = Point2::new(
			state.buffer_width() as f32 / 2.,
			state.buffer_height() as f32 / 2.,
		);
		let ratio = utils::min(1., (state.time() - self.start_time) / 4.) as f32;
		game::draw_blade_blade(
			center,
			0.,
			state.buffer_width() as f32 / 90.,
			10,
			0.06,
			ratio,
			3.,
			state,
		);
		let sprite = state.get_sprite("data/logo.cfg").unwrap();
		sprite.draw_frame(center, "Default", 0, state);

		let rc_buffer = game_state::light_pass(state);

		state.core.set_target_bitmap(state.buffer1.as_ref());
		state.core.clear_to_color(Color::from_rgb_f(0., 0., 0.));
		state
			.core
			.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::InverseAlpha);

		if state.time() - self.start_time > 0.5
		{
			let f = utils::clamp(((state.time() - self.start_time - 0.5) / 2.) as f32, 0., 1.);
			state.core.draw_tinted_bitmap(
				rc_buffer.unwrap(),
				Color::from_rgb_f(f, f, f),
				0.,
				0.,
				Flag::zero(),
			);
		}

		if !self.subscreens.is_empty()
		{
			self.subscreens.draw(state);
		}

		Ok(())
	}

	pub fn resize(&mut self, state: &game_state::GameState)
	{
		if !self.subscreens.is_empty()
		{
			self.subscreens.resize(state);
		}
	}
}
