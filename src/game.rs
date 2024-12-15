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
use tiled;

use std::collections::HashMap;
use std::path::Path;

const TILE_SIZE: i32 = 32;

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

struct MapChunk
{
	tiles: Vec<i32>,
	width: i32,
	height: i32,
}

impl MapChunk
{
	pub fn new(filename: &str) -> Result<Self>
	{
		let map = tiled::Loader::new().load_tmx_map(&Path::new(&filename))?;
		let layer_tiles = match map.get_layer(0).unwrap().layer_type()
		{
			tiled::LayerType::Tiles(layer_tiles) => layer_tiles,
			_ => return Err("Layer 0 must be the tile layer!".to_string().into()),
		};

		let height = layer_tiles.height().unwrap() as usize;
		let width = layer_tiles.width().unwrap() as usize;

		let mut tiles = Vec::with_capacity(width * height);

		for y in 0..height
		{
			for x in 0..width
			{
				let id = layer_tiles.get_tile(x as i32, y as i32).unwrap().id();
				tiles.push(id as i32);
			}
		}

		Ok(Self {
			tiles: tiles,
			width: width as i32,
			height: height as i32,
		})
	}

	pub fn draw(&self, state: &game_state::GameState) -> Result<()>
	{
		let sprite = state.get_sprite("data/terrain.cfg")?;
		for y in 0..self.height
		{
			for x in 0..self.width
			{
				let tile_size = TILE_SIZE as f32;
				let tile_idx = self.tiles[y as usize * self.height as usize + x as usize];
				sprite.draw_frame(
					Point2::new(x as f32 * tile_size, y as f32 * tile_size),
					"Default",
					tile_idx,
					state,
				);
			}
		}
		Ok(())
	}
}

struct Map
{
	world: hecs::World,
	player: hecs::Entity,
	chunks: Vec<MapChunk>,
	camera_pos: Point2<f32>,
}

impl Map
{
	fn new(state: &mut game_state::GameState) -> Result<Self>
	{
		let mut world = hecs::World::new();

		let spawn_pos = Point2::new(100., 100.);

		let player = spawn_obj(spawn_pos, &mut world)?;

		state.cache_sprite("data/terrain.cfg")?;

		Ok(Self {
			world: world,
			player: player,
			chunks: vec![MapChunk::new("data/test.tmx")?],
			camera_pos: spawn_pos,
		})
	}

	fn camera_to_world(&self, pos: Point2<f32>, state: &game_state::GameState) -> Point2<f32>
	{
		self.camera_pos + pos.coords
			- Vector2::new(state.buffer_width() / 2., state.buffer_height() / 2.)
	}

	fn camera_transform(&self, state: &game_state::GameState) -> Transform
	{
		let mut transform = Transform::identity();
		transform.translate(-self.camera_pos.x, -self.camera_pos.y);
		transform.translate(state.buffer_width() / 2., state.buffer_height() / 2.);
		transform
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
			let diff = self.camera_to_world(mouse_pos.cast::<f32>(), state) - position.pos;
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
		state.core.clear_to_color(Color::from_rgb_f(0.3, 0.3, 0.3));

		if let Ok(pos) = self.world.get::<&comps::Position>(self.player)
		{
			self.camera_pos = utils::round_point(pos.draw_pos(state.alpha));
		}
		state.core.use_transform(&self.camera_transform(state));

		state
			.core
			.use_shader(Some(&*state.basic_shader.upgrade().unwrap()))
			.unwrap();

		for chunk in self.chunks.iter_mut()
		{
			chunk.draw(state)?;
		}

		state
			.core
			.use_shader(Some(&*state.palette_shader.upgrade().unwrap()))
			.unwrap();

		// Drawable
		for (_, (drawable, position)) in self
			.world
			.query::<(&comps::Drawable, &comps::Position)>()
			.iter()
		{
			let sprite = state.get_sprite(&drawable.sprite)?;
			let palette_index = state.palettes.get_palette_index(
				drawable
					.palette
					.as_ref()
					.unwrap_or(&sprite.get_palettes()[0]),
			)?;

			state
				.core
				.set_shader_uniform("palette_index", &[palette_index as f32][..])
				.ok();
			state
				.core
				.set_shader_sampler("palette", &state.palettes.palette_bitmap, 2)
				.ok();

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
