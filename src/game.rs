use crate::error::Result;
use crate::utils::DT;
use crate::{astar, components as comps, controls, game_state, sprite, ui, utils};
use allegro::*;
use allegro_font::*;
use na::{
	Isometry3, Matrix4, Perspective3, Point2, Point3, Quaternion, RealField, Rotation2, Rotation3,
	Unit, Vector2, Vector3, Vector4,
};
use nalgebra as na;
use rand::prelude::*;

use std::collections::HashMap;

pub struct Game
{
	map: Map,
	subscreens: ui::SubScreens,
}

impl Game
{
	pub fn new(state: &mut game_state::GameState) -> Result<Self>
	{
		state.cache_sprite("data/player_anim.cfg")?;

		Ok(Self {
			map: Map::new(state)?,
			subscreens: ui::SubScreens::new(state),
		})
	}

	pub fn logic(
		&mut self, state: &mut game_state::GameState,
	) -> Result<Option<game_state::NextScreen>>
	{
		if self.subscreens.is_empty()
		{
			self.map.logic(state)
		}
		else
		{
			Ok(None)
		}
	}

	pub fn input(
		&mut self, event: &Event, state: &mut game_state::GameState,
	) -> Result<Option<game_state::NextScreen>>
	{
		state.controls.decode_event(event);
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
			_ => (),
		}
		if self.subscreens.is_empty()
		{
			let in_game_menu;
			match *event
			{
				Event::KeyDown {
					keycode: KeyCode::Escape,
					..
				} =>
				{
					in_game_menu = true;
				}
				_ =>
				{
					let res = self.map.input(event, state);
					if let Ok(Some(game_state::NextScreen::InGameMenu)) = res
					{
						in_game_menu = true;
					}
					else
					{
						return res;
					}
				}
			}
			if in_game_menu
			{
				self.subscreens
					.push(ui::SubScreen::InGameMenu(ui::InGameMenu::new(state)));
				self.subscreens.reset_transition(state);
				state.paused = true;
			}
		}
		else
		{
			if let Some(action) = self.subscreens.input(state, event)?
			{
				match action
				{
					ui::Action::MainMenu => return Ok(Some(game_state::NextScreen::Menu)),
					_ => (),
				}
			}
			if self.subscreens.is_empty()
			{
				state.paused = false;
			}
		}
		Ok(None)
	}

	pub fn draw(&mut self, state: &game_state::GameState) -> Result<()>
	{
		if !self.subscreens.is_empty()
		{
			state.core.clear_to_color(Color::from_rgb_f(0.0, 0.0, 0.0));
			self.subscreens.draw(state);
		}
		else
		{
			self.map.draw(state)?;
		}
		Ok(())
	}

	pub fn resize(&mut self, state: &game_state::GameState)
	{
		self.subscreens.resize(state);
	}
}

pub fn spawn_obj(pos: Point2<f32>, world: &mut hecs::World) -> Result<hecs::Entity>
{
	let entity = world.spawn((
		comps::Drawable::new("data/player_anim.cfg"),
		comps::Position::new(pos),
		comps::Velocity {
			pos: Vector2::new(0., 0.),
		},
	));
	Ok(entity)
}

fn vec_to_dir_name(vec: Vector2<f32>) -> &'static str
{
	match (vec.x > 0., vec.y > 0., vec.x.abs() > vec.y.abs())
	{
		(true, _, true) => "Right",
		(false, _, true) => "Left",
		(_, true, false) => "Down",
		(_, false, false) => "Up",
	}
}

struct Map
{
	world: hecs::World,
	player: hecs::Entity,
}

impl Map
{
	fn new(_state: &mut game_state::GameState) -> Result<Self>
	{
		let mut world = hecs::World::new();
		let player = spawn_obj(Point2::new(100., 100.), &mut world)?;

		Ok(Self {
			world: world,
			player: player,
		})
	}

	fn logic(&mut self, state: &mut game_state::GameState)
		-> Result<Option<game_state::NextScreen>>
	{
		let mut to_die = vec![];

		// Position snapshotting.
		for (_, position) in self.world.query::<&mut comps::Position>().iter()
		{
			position.snapshot();
		}

		// Input.
		// if state.controls.get_action_state(controls::Action::Move) > 0.5
		// {
		// 	for (_, position) in self.world.query::<&mut comps::Position>().iter()
		// 	{
		// 		position.pos.y += 100. * DT;
		// 	}
		// }
		let mouse_pos = state.mouse_pos;
		for (_, (position, velocity)) in self
			.world
			.query::<(&mut comps::Position, &mut comps::Velocity)>()
			.iter()
		{
			let diff = mouse_pos.cast::<f32>() - position.pos;
			let norm_diff = diff.normalize();
			if diff.norm() < 100.
			{
				velocity.pos = Vector2::new(0., 0.);
			}
			else
			{
				velocity.pos = 200. * norm_diff;
			}
			position.dir = norm_diff.y.atan2(norm_diff.x);
		}

		// Drawable animation selection.
		for (_, (drawable, position, velocity)) in self
			.world
			.query::<(&mut comps::Drawable, &comps::Position, &comps::Velocity)>()
			.iter()
		{
			if velocity.pos.norm() > 0.
			{
				drawable.animation_name = format!("Move{}", vec_to_dir_name(velocity.pos));
			}
			else
			{
				let dir = Vector2::new(position.dir.cos(), position.dir.sin());
				drawable.animation_name = format!("Stand{}", vec_to_dir_name(dir));
			}
		}

		// Movement.
		for (_, (position, velocity)) in self
			.world
			.query::<(&mut comps::Position, &comps::Velocity)>()
			.iter()
		{
			position.pos += DT * velocity.pos;
		}

		// Remove dead entities
		to_die.sort();
		to_die.dedup();
		for id in to_die
		{
			//println!("died {id:?}");
			self.world.despawn(id)?;
		}

		Ok(None)
	}

	fn input(
		&mut self, _event: &Event, _state: &mut game_state::GameState,
	) -> Result<Option<game_state::NextScreen>>
	{
		Ok(None)
	}

	fn draw(&mut self, state: &game_state::GameState) -> Result<()>
	{
		state.core.clear_to_color(Color::from_rgb_f(0., 0.0, 0.1));

		// Drawable
		for (_, (drawable, position)) in self
			.world
			.query::<(&comps::Drawable, &comps::Position)>()
			.iter()
		{
			let sprite = state.get_sprite(&drawable.sprite)?;
			sprite.draw(
				position.draw_pos(state.alpha),
				&drawable.animation_name,
				state.time() - drawable.animation_start,
				drawable.animation_speed,
				&state,
			);
		}

		Ok(())
	}
}
