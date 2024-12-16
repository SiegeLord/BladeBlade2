use crate::components::Velocity;
use crate::error::Result;
use crate::utils::DT;
use crate::{astar, components as comps, controls, game_state, spatial_grid, sprite, ui, utils};
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

const TILE_SIZE: f32 = 32.;

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
		state.cache_sprite("data/shadow.cfg")?;

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

pub fn spawn_player(pos: Point2<f32>, world: &mut hecs::World) -> Result<hecs::Entity>
{
	let entity = world.spawn((
		comps::Drawable::new("data/player_anim.cfg"),
		comps::Position::new(pos),
		comps::Velocity {
			pos: Vector2::zeros(),
		},
		comps::Acceleration {
			pos: Vector2::zeros(),
		},
		comps::Solid {
			size: 8.,
			mass: 1.,
			collision_class: comps::CollisionClass::Big,
		},
	));
	Ok(entity)
}

pub fn spawn_enemy(pos: Point2<f32>, world: &mut hecs::World) -> Result<hecs::Entity>
{
	let mut drawable = comps::Drawable::new("data/player_anim.cfg");
	drawable.palette = Some("data/player_pal2.png".to_string());
	let entity = world.spawn((
		drawable,
		comps::Position::new(pos),
		comps::Velocity {
			pos: Vector2::zeros(),
		},
		comps::Acceleration {
			pos: Vector2::zeros(),
		},
		comps::Solid {
			size: 8.,
			mass: 1.,
			collision_class: comps::CollisionClass::Big,
		},
		comps::AI::new(),
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

#[derive(Debug, Copy, Clone)]
struct GridInner
{
	id: hecs::Entity,
	pos: Point2<f32>,
}

struct MapChunk
{
	tiles: Vec<i32>,
	width: i32,
	height: i32,
}

impl MapChunk
{
	fn new(filename: &str) -> Result<Self>
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

	fn draw(&self, state: &game_state::GameState) -> Result<()>
	{
		let sprite = state.get_sprite("data/terrain.cfg")?;
		for y in 0..self.height
		{
			for x in 0..self.width
			{
				let tile_idx = self.tiles[y as usize * self.height as usize + x as usize];
				sprite.draw_frame(
					Point2::new(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE),
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

		let player = spawn_player(spawn_pos, &mut world)?;

		for i in 0..3
		{
			for j in 0..3
			{
				spawn_enemy(
					Point2::new(200. + i as f32 * 32., 200. + j as f32 * 32.),
					&mut world,
				)?;
			}
		}

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
		if let Ok((_position, acceleration)) = self
			.world
			.query_one_mut::<(&comps::Position, &mut comps::Acceleration)>(self.player)
		{
			let dx = state.controls.get_action_state(controls::Action::MoveRight)
				- state.controls.get_action_state(controls::Action::MoveLeft);
			let dy = state.controls.get_action_state(controls::Action::MoveDown)
				- state.controls.get_action_state(controls::Action::MoveUp);

			acceleration.pos = Vector2::new(dx, dy) * 1024.;
		}

		// AI
		let mut rng = thread_rng();
		for (_, (position, acceleration, ai)) in self
			.world
			.query::<(&comps::Position, &mut comps::Acceleration, &mut comps::AI)>()
			.iter()
		{
			let idle_time = 3.;
			let wander_time = 0.5;
			let chase_time = 1.;
			let attack_time = 1.;
			let acceleration_amount = 64.;
			let sense_range = 128.;

			// TODO: Better target acquisition.
			let mut target;
			if let Some(cur_target) = ai.state.get_target()
			{
				target = Some(cur_target);
			}
			else
			{
				target = Some(self.player);
			}

			let target_position =
				target.and_then(|target| self.world.get::<&comps::Position>(target).ok());
			if let Some(target_position) = target_position.as_ref()
			{
				let dist = (target_position.pos - position.pos).norm();
				if dist > sense_range
				{
					target = None;
				}
			}

			let mut next_state = None;
			match ai.state
			{
				comps::AIState::Idle =>
				{
					if let Some(target) = target
					{
						next_state = Some(comps::AIState::Chase(target));
					}
					else
					{
						acceleration.pos = Vector2::zeros();
					}
					if state.time() > ai.next_state_time
					{
						let next_state_weight =
							[(comps::AIState::Idle, 1), (comps::AIState::Wander, 1)];
						next_state = next_state_weight
							.choose_weighted(&mut rng, |sw| sw.1)
							.ok()
							.map(|sw| sw.0);
						match ai.state
						{
							comps::AIState::Wander =>
							{
								let dir_x = rng.gen_range(-1..=1) as f32;
								let dir_y = rng.gen_range(-1..=1) as f32;
								acceleration.pos = Vector2::new(dir_x, dir_y) * acceleration_amount;
							}
							_ => (),
						}
					}
				}
				comps::AIState::Wander =>
				{
					if let Some(target) = target
					{
						next_state = Some(comps::AIState::Chase(target));
					}
					if state.time() > ai.next_state_time
					{
						next_state = Some(comps::AIState::Idle);
					}
				}
				comps::AIState::Chase(_) =>
				{
					if let Some(target_position) = target_position
					{
						let diff = (target_position.pos - position.pos).normalize();
						acceleration.pos = diff * acceleration_amount;
					}
					if state.time() > ai.next_state_time
					{
						if let Some(target) = target
						{
							next_state = Some(comps::AIState::Chase(target));
						}
						else
						{
							next_state = Some(comps::AIState::Idle);
						}
					}
				}
				_ => (),
			}
			if let Some(next_state) = next_state
			{
				match next_state
				{
					comps::AIState::Idle =>
					{
						ai.next_state_time = state.time() + idle_time;
					}
					comps::AIState::Wander =>
					{
						ai.next_state_time = state.time() + wander_time;
					}
					comps::AIState::Chase(_) =>
					{
						ai.next_state_time = state.time() + chase_time;
					}
					comps::AIState::Attack(_) =>
					{
						ai.next_state_time = state.time() + attack_time;
					}
				}
				ai.state = next_state;
			}
		}

		// Drawable animation state handling.
		for (_, (drawable, position, acceleration, velocity)) in self
			.world
			.query::<(
				&mut comps::Drawable,
				&comps::Position,
				&comps::Acceleration,
				&comps::Velocity,
			)>()
			.iter()
		{
			let sprite = state.get_sprite(&drawable.sprite)?;
			let mut speed = 1.;
			if acceleration.pos.norm() > 0.
			{
				drawable
					.animation_state
					.set_animation(format!("Move{}", vec_to_dir_name(acceleration.pos)));
				speed = velocity.pos.norm() / 256.;
			}
			else
			{
				let dir = Vector2::new(position.dir.cos(), position.dir.sin());
				drawable
					.animation_state
					.set_animation(format!("Stand{}", vec_to_dir_name(dir)));
			}
			sprite.advance_state(&mut drawable.animation_state, (speed * DT) as f64);
		}

		// Movement.
		for (_, (acceleration, velocity)) in self
			.world
			.query::<(&mut comps::Acceleration, &mut comps::Velocity)>()
			.iter()
		{
			let decel = 1024.;
			if velocity.pos.x.abs() > 0. && acceleration.pos.x == 0.
			{
				if velocity.pos.x.abs() <= decel * DT
				{
					velocity.pos.x = 0.
				}
				else
				{
					acceleration.pos.x = -velocity.pos.x.signum() * decel;
				}
			}
			if velocity.pos.y.abs() > 0. && acceleration.pos.y == 0.
			{
				if velocity.pos.y.abs() <= decel * DT
				{
					velocity.pos.y = 0.
				}
				else
				{
					acceleration.pos.y = -velocity.pos.y.signum() * decel;
				}
			}
		}

		for (_, (velocity, acceleration)) in self
			.world
			.query::<(&mut comps::Velocity, &comps::Acceleration)>()
			.iter()
		{
			let max_vel = 196.;
			velocity.pos = velocity.pos + DT * acceleration.pos;
			if acceleration.pos.norm() > 0.
			{
				let projected_speed = velocity.pos.dot(&acceleration.pos.normalize());
				if projected_speed > max_vel
				{
					velocity.pos *= max_vel / projected_speed;
				}
			}
		}

		for (_id, (position, velocity)) in self
			.world
			.query::<(&mut comps::Position, &comps::Velocity)>()
			.iter()
		{
			position.pos += DT * velocity.pos;
			if velocity.pos.norm() > 0.
			{
				position.dir = velocity.pos.y.atan2(velocity.pos.x);
			}
		}

		// Collision detection
		let mut grid = spatial_grid::SpatialGrid::new(
			self.chunks[0].width as usize,
			self.chunks[0].height as usize,
			TILE_SIZE,
			TILE_SIZE,
		);

		for (id, (position, solid)) in self.world.query_mut::<(&comps::Position, &comps::Solid)>()
		{
			let margin = 8.;
			let r = solid.size + margin;
			let x = position.pos.x;
			let y = position.pos.y;
			grid.push(spatial_grid::entry(
				Point2::new(x - r, y - r),
				Point2::new(x + r, y + r),
				GridInner {
					pos: position.pos,
					id: id,
				},
			));
		}

		let mut colliding_pairs = vec![];
		for (a, b) in grid.all_pairs(|a, b| {
			let a_solid = self.world.get::<&comps::Solid>(a.inner.id).unwrap();
			let b_solid = self.world.get::<&comps::Solid>(b.inner.id).unwrap();
			a_solid
				.collision_class
				.collides_with(b_solid.collision_class)
		})
		{
			colliding_pairs.push((a.inner, b.inner));
		}
		for _pass in 0..5
		{
			for &(inner1, inner2) in &colliding_pairs
			{
				let id1 = inner1.id;
				let id2 = inner2.id;
				let pos1 = self.world.get::<&comps::Position>(id1)?.pos;
				let pos2 = self.world.get::<&comps::Position>(id2)?.pos;

				let solid1 = *self.world.get::<&comps::Solid>(id1)?;
				let solid2 = *self.world.get::<&comps::Solid>(id2)?;

				let diff = pos2.xy() - pos1.xy();
				let diff_norm = utils::max(0.1, diff.norm());

				if diff_norm > solid1.size + solid2.size
				{
					continue;
				}

				if solid1.collision_class.interacts() && solid2.collision_class.interacts()
				{
					let diff = 0.9 * diff * (solid1.size + solid2.size - diff_norm) / diff_norm;

					let f1 = 1. - solid1.mass / (solid2.mass + solid1.mass);
					let f2 = 1. - solid2.mass / (solid2.mass + solid1.mass);
					if f32::is_finite(f1)
					{
						self.world.get::<&mut comps::Position>(id1)?.pos -= diff * f1;
					}
					if f32::is_finite(f2)
					{
						self.world.get::<&mut comps::Position>(id2)?.pos += diff * f2;
					}
				}
			}
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

		let mut drawables = vec![];
		// Drawable
		for (_, (drawable, position)) in self
			.world
			.query_mut::<(&comps::Drawable, &comps::Position)>()
		{
			drawables.push((drawable, position));
		}

		drawables.sort_by_key(|(_, position)| position.pos.y as i32);

		for (_, position) in &drawables
		{
			let sprite = state.get_sprite("data/shadow.cfg")?;

			sprite.draw_frame(
				utils::round_point(position.draw_pos(state.alpha)),
				"Default",
				0,
				&state,
			);
			// TODO: Why are primitives broken?
			//state.prim.draw_filled_circle(
			//	position.pos.x,
			//	position.pos.y,
			//	16.,
			//	Color::from_rgba_f(1., 1., 1., 1.),
			//);
		}

		state
			.core
			.use_shader(Some(&*state.palette_shader.upgrade().unwrap()))
			.unwrap();
		for (drawable, position) in drawables
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
				utils::round_point(position.draw_pos(state.alpha)),
				&drawable.animation_state,
				&state,
			);
		}

		Ok(())
	}
}
