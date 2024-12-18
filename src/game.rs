use crate::components::Velocity;
use crate::error::Result;
use crate::utils::{XYExt, DT};
use crate::{
	astar, atlas, components as comps, controls, game_state, spatial_grid, sprite, ui, utils,
};
use allegro::*;
use allegro_font::*;
use allegro_primitives::*;
use na::{
	Isometry3, Matrix4, Perspective3, Point2, Point3, Quaternion, RealField, Rotation2, Rotation3,
	Unit, Vector2, Vector3, Vector4,
};
use nalgebra::{self as na, Point};
use rand::prelude::*;
use tiled;

use std::collections::HashMap;
use std::path::Path;

const TILE_SIZE: f32 = 32.;
const PI: f32 = std::f32::consts::PI;

pub struct Game
{
	map: Map,
	subscreens: ui::SubScreens,
}

impl Game
{
	pub fn new(state: &mut game_state::GameState) -> Result<Self>
	{
		state.cache_sprite("data/player.cfg")?;
		state.cache_sprite("data/fireball.cfg")?;
		state.cache_sprite("data/fire_hit.cfg")?;
		state.cache_sprite("data/shadow.cfg")?;
		state.cache_sprite("data/terrain.cfg")?;
		state.cache_sprite("data/crystal_red.cfg")?;
		state.cache_sprite("data/crystal_blue.cfg")?;
		state.cache_sprite("data/crystal_green.cfg")?;
		state.cache_sprite("data/crystal_pips.cfg")?;
		state.cache_sprite("data/soul.cfg")?;
		state.cache_sprite("data/power_sphere.cfg")?;

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

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct BladeVertex
{
	x: f32,
	y: f32,
	z: f32,
	u: f32,
	v: f32,
	palette_index: f32,
	material: f32,
	color: Color,
}

unsafe impl VertexType for BladeVertex
{
	fn get_decl(prim: &PrimitivesAddon) -> VertexDecl
	{
		fn make_builder() -> std::result::Result<VertexDeclBuilder, ()>
		{
			VertexDeclBuilder::new(std::mem::size_of::<BladeVertex>())
				.pos(
					VertexAttrStorage::F32_3,
					memoffset::offset_of!(BladeVertex, x),
				)?
				.uv(
					VertexAttrStorage::F32_2,
					memoffset::offset_of!(BladeVertex, u),
				)?
				.color(memoffset::offset_of!(BladeVertex, color))?
				.user_attr(
					VertexAttrStorage::F32_2,
					memoffset::offset_of!(BladeVertex, palette_index),
				)
		}

		VertexDecl::from_builder(prim, &make_builder().unwrap())
	}
}

struct Bucket
{
	vertices: Vec<BladeVertex>,
	indices: Vec<i32>,
}

struct Scene
{
	buckets: Vec<Bucket>,
}

impl Scene
{
	fn new() -> Self
	{
		Scene { buckets: vec![] }
	}

	fn ensure_bucket(&mut self, page: usize)
	{
		while page >= self.buckets.len()
		{
			self.buckets.push(Bucket {
				vertices: vec![],
				indices: vec![],
			});
		}
	}

	fn add_vertices(&mut self, vertices: &[BladeVertex], page: usize)
	{
		self.ensure_bucket(page);
		self.buckets[page].vertices.extend(vertices);
	}

	fn add_indices(&mut self, indices: &[i32], page: usize)
	{
		self.ensure_bucket(page);
		self.buckets[page].indices.extend(indices);
	}

	fn add_vertex(&mut self, vertex: BladeVertex, page: usize)
	{
		self.ensure_bucket(page);
		self.buckets[page].vertices.push(vertex);
	}

	fn add_index(&mut self, index: i32, page: usize)
	{
		self.ensure_bucket(page);
		self.buckets[page].indices.push(index);
	}

	fn num_vertices(&mut self, page: usize) -> i32
	{
		self.ensure_bucket(page);
		self.buckets[page].vertices.len() as i32
	}

	fn add_bitmap(
		&mut self, pos: Point3<f32>, bmp: atlas::AtlasBitmap, palette_index: i32, material: i32,
	)
	{
		let color = Color::from_rgb_f(1., 1., 1.);
		let page_size = 1024.;
		let vertices = [
			BladeVertex {
				x: pos.x,
				y: pos.y,
				z: pos.z,
				u: bmp.start.x / page_size,
				v: 1. - bmp.start.y / page_size,
				color: color,
				palette_index: palette_index as f32,
				material: material as f32,
			},
			BladeVertex {
				x: pos.x + bmp.width(),
				y: pos.y,
				z: pos.z,
				u: bmp.end.x / page_size,
				v: 1. - bmp.start.y / page_size,
				color: color,
				palette_index: palette_index as f32,
				material: material as f32,
			},
			BladeVertex {
				x: pos.x + bmp.width(),
				y: pos.y + bmp.height(),
				z: pos.z,
				u: bmp.end.x / page_size,
				v: 1. - bmp.end.y / page_size,
				color: color,
				palette_index: palette_index as f32,
				material: material as f32,
			},
			BladeVertex {
				x: pos.x,
				y: pos.y + bmp.height(),
				z: pos.z,
				u: bmp.start.x / page_size,
				v: 1. - bmp.end.y / page_size,
				color: color,
				palette_index: palette_index as f32,
				material: material as f32,
			},
		];
		let idx = self.num_vertices(bmp.page);
		let indices = [idx + 0, idx + 1, idx + 2, idx + 0, idx + 2, idx + 3];
		self.add_indices(&indices[..], bmp.page);
		self.add_vertices(&vertices[..], bmp.page);
	}
}

fn spawn_player(pos: Point3<f32>, world: &mut hecs::World) -> Result<hecs::Entity>
{
	let entity = world.spawn((
		comps::Appearance::new("data/player.cfg"),
		comps::Position::new(pos),
		comps::Velocity {
			pos: Vector3::zeros(),
		},
		comps::Acceleration {
			pos: Vector3::zeros(),
		},
		comps::Solid {
			size: 8.,
			mass: 1.,
			kind: comps::CollisionKind::BigPlayer,
		},
		comps::Stats::new(comps::StatValues::new_player()),
		comps::Attack::new(comps::AttackKind::BladeBlade),
		comps::Jump::new(),
		comps::AffectedByGravity::new(),
		comps::BladeBlade::new(),
		comps::CastsShadow,
		comps::Controller::new(),
		comps::OnDeathEffect {
			effects: vec![comps::Effect::SpawnCorpse],
		},
	));
	Ok(entity)
}

fn spawn_enemy(
	pos: Point3<f32>, crystal_id: hecs::Entity, world: &mut hecs::World,
) -> Result<hecs::Entity>
{
	let mut appearance = comps::Appearance::new("data/player.cfg");
	appearance.palette = Some("data/player_pal2.png".to_string());
	let entity = world.spawn((
		appearance,
		comps::Position::new(pos),
		comps::Velocity {
			pos: Vector3::zeros(),
		},
		comps::Acceleration {
			pos: Vector3::zeros(),
		},
		comps::Solid {
			size: 8.,
			mass: 1.,
			kind: comps::CollisionKind::BigEnemy,
		},
		comps::AI::new(),
		comps::Stats::new(comps::StatValues::new_enemy()),
		comps::Attack::new(comps::AttackKind::Fireball),
		comps::CastsShadow,
		comps::Controller::new(),
		comps::OnDeathEffect {
			effects: vec![
				comps::Effect::SpawnCorpse,
				comps::Effect::SpawnSoul(crystal_id),
			],
		},
	));
	Ok(entity)
}

fn spawn_crystal(
	pos: Point3<f32>, kind: comps::ItemKind, world: &mut hecs::World,
) -> Result<hecs::Entity>
{
	let sprite = match kind
	{
		comps::ItemKind::Blue => "data/crystal_blue.cfg",
		comps::ItemKind::Red => "data/crystal_red.cfg",
		comps::ItemKind::Green => "data/crystal_green.cfg",
	};
	let entity = world.spawn((
		comps::Appearance::new(sprite),
		comps::Position::new(pos),
		comps::Solid {
			size: 8.,
			mass: std::f32::INFINITY,
			kind: comps::CollisionKind::World,
		},
		comps::CastsShadow,
		comps::Crystal::new(kind),
		comps::OnDeathEffect {
			effects: vec![comps::Effect::SpawnPowerSphere(kind)],
		},
	));
	Ok(entity)
}

fn spawn_from_crystal(id: hecs::Entity, world: &mut hecs::World) -> Result<()>
{
	let mut vals = None;
	if let Ok((position, crystal)) = world.query_one_mut::<(&comps::Position, &comps::Crystal)>(id)
	{
		vals = Some((position.pos, crystal.level));
	}

	let count = if let Some((pos, level)) = vals
	{
		let count = 1 + level;
		let mut rng = thread_rng();

		for _ in 0..count
		{
			spawn_enemy(
				pos + Vector3::new(rng.gen_range(-5.0..5.0), rng.gen_range(-5.0..5.0), 0.0),
				id,
				world,
			)?;
		}
		count
	}
	else
	{
		0
	};

	if vals.is_some()
	{
		world
			.query_one_mut::<&mut comps::Crystal>(id)
			.unwrap()
			.enemies += count;
	}
	Ok(())
}

fn spawn_corpse(
	pos: Point3<f32>, vel_pos: Vector3<f32>, appearance: comps::Appearance, world: &mut hecs::World,
) -> Result<hecs::Entity>
{
	let entity = world.spawn((
		appearance,
		comps::Position::new(pos),
		comps::Velocity { pos: vel_pos },
		comps::Acceleration {
			pos: Vector3::zeros(),
		},
		comps::AffectedByGravity::new(),
		comps::CastsShadow,
		comps::Corpse,
		comps::Solid {
			size: 8.,
			mass: 1.,
			kind: comps::CollisionKind::SmallPlayer,
		},
		// TODO: Surprising amount of components to get friction/gravity working
		comps::Stats::new(comps::StatValues::new_player()),
		comps::Controller::new(),
	));
	Ok(entity)
}

fn spawn_fireball(
	pos: Point3<f32>, velocity_pos: Vector3<f32>, acceleration_pos: Vector3<f32>, time: f64,
	world: &mut hecs::World,
) -> Result<hecs::Entity>
{
	let team = comps::Team::Enemy;
	let entity = world.spawn((
		comps::Appearance::new("data/fireball.cfg"),
		comps::Position::new(pos),
		comps::Velocity { pos: velocity_pos },
		comps::Acceleration {
			pos: acceleration_pos,
		},
		comps::Solid {
			size: 8.,
			mass: 0.,
			kind: comps::CollisionKind::SmallEnemy,
		},
		comps::Stats::new(comps::StatValues::new_fireball()),
		comps::TimeToDie::new(time + 1.),
		comps::OnContactEffect {
			effects: vec![
				comps::Effect::Die,
				comps::Effect::SpawnFireHit,
				comps::Effect::DoDamage(comps::Damage::Magic(1.), team),
			],
		},
		comps::OnDeathEffect {
			effects: vec![comps::Effect::SpawnFireHit],
		},
		comps::CastsShadow,
	));
	Ok(entity)
}

fn spawn_soul(
	pos: Point3<f32>, target: Point3<f32>, crystal_id: hecs::Entity, world: &mut hecs::World,
) -> Result<hecs::Entity>
{
	let entity = world.spawn((
		comps::Appearance::new("data/soul.cfg"),
		comps::Position::new(pos),
		comps::Velocity {
			pos: Vector3::zeros(),
		},
		comps::Acceleration {
			pos: 256. * (target - pos).normalize(),
		},
		comps::Stats::new(comps::StatValues::new_fireball()),
		comps::OnDeathEffect {
			effects: vec![comps::Effect::UnlockCrystal(crystal_id)],
		},
		comps::PlaceToDie::new(target),
	));
	Ok(entity)
}

fn spawn_power_sphere(
	pos: Point3<f32>, target: Point3<f32>, crystal_id: hecs::Entity, world: &mut hecs::World,
) -> Result<hecs::Entity>
{
	let entity = world.spawn((
		comps::Appearance::new("data/power_sphere.cfg"),
		comps::Position::new(pos),
		comps::Velocity {
			pos: Vector3::zeros(),
		},
		comps::Acceleration {
			pos: 256. * (target - pos).normalize(),
		},
		comps::Stats::new(comps::StatValues::new_fireball()),
		comps::OnDeathEffect {
			effects: vec![comps::Effect::ElevateCrystal(crystal_id)],
		},
		comps::PlaceToDie::new(target),
	));
	Ok(entity)
}

fn spawn_fire_hit(pos: Point3<f32>, world: &mut hecs::World) -> Result<hecs::Entity>
{
	let mut appearance = comps::Appearance::new("data/fire_hit.cfg");
	appearance.bias = 1;
	let entity = world.spawn((
		appearance,
		comps::Position::new(pos),
		comps::DieOnActivation,
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
	pos: Point3<f32>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TileKind
{
	Empty,
	Floor,
}

impl TileKind
{
	fn from_id(id: i32) -> Self
	{
		match id
		{
			0 => TileKind::Empty,
			_ => TileKind::Floor,
		}
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

	fn draw(
		&self, pos: Point2<f32>, scene: &mut Scene, z_shift: f32, state: &game_state::GameState,
	) -> Result<()>
	{
		let sprite = state.get_sprite("data/terrain.cfg")?;
		let palette_index = state
			.palettes
			.get_palette_index(&sprite.get_palettes()[0])?;
		for y in 0..self.height
		{
			for x in 0..self.width
			{
				let tile_idx = self.tiles[y as usize * self.height as usize + x as usize];
				if tile_idx == 0
				{
					continue;
				}
				let (atlas_bmp, offt) = sprite.get_frame("Default", tile_idx);

				let tile_pos = Vector2::new(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE);
				let pos = utils::round_point(pos + tile_pos) + offt;
				scene.add_bitmap(
					Point3::new(pos.x, pos.y, tile_pos.y + z_shift),
					atlas_bmp,
					palette_index,
					0,
				);
			}
		}
		Ok(())
	}

	fn get_tile_kind(&self, pos: Point2<f32>) -> TileKind
	{
		let tile_x = ((pos.x + TILE_SIZE / 2.) / TILE_SIZE).floor() as i32;
		let tile_y = ((pos.y + TILE_SIZE / 2.) / TILE_SIZE).floor() as i32;
		if tile_x < 0 || tile_x >= self.width || tile_y < 0 || tile_y >= self.height
		{
			return TileKind::Empty;
		}
		TileKind::from_id(self.tiles[tile_y as usize * self.width as usize + tile_x as usize])
	}

	pub fn get_escape_dir(
		&self, pos: Point2<f32>, size: f32, avoid_kind: TileKind,
	) -> Option<Vector2<f32>>
	{
		let tile_x = ((pos.x + TILE_SIZE / 2.) / TILE_SIZE) as i32;
		let tile_y = ((pos.y + TILE_SIZE / 2.) / TILE_SIZE) as i32;

		let mut res = Vector2::zeros();
		// TODO: This -1/1 isn't really right (???)
		for map_y in tile_y - 1..=tile_y + 1
		{
			for map_x in tile_x - 1..=tile_x + 1
			{
				if map_x < 0 || map_x >= self.width || map_y < 0 || map_y >= self.height
				{
					continue;
				}
				let tile = self.tiles[(map_y * self.width + map_x) as usize];
				if TileKind::from_id(tile) != avoid_kind
				{
					continue;
				}

				let cx = map_x as f32 * TILE_SIZE - TILE_SIZE / 2.;
				let cy = map_y as f32 * TILE_SIZE - TILE_SIZE / 2.;

				let vs = [
					Point2::new(cx, cy),
					Point2::new(cx, cy + TILE_SIZE),
					Point2::new(cx + TILE_SIZE, cy + TILE_SIZE),
					Point2::new(cx + TILE_SIZE, cy),
				];

				let nearest_point = utils::nearest_poly_point(&vs, pos);

				let nearest_dist = utils::max(1e-20, (pos - nearest_point).norm());
				let inside = utils::is_inside_poly(&vs, pos);
				if nearest_dist < size || inside
				{
					let new_dir = if inside
					{
						(nearest_point - pos) * (nearest_dist + size) / nearest_dist
					}
					else
					{
						(pos - nearest_point) * (size - nearest_dist) / nearest_dist
					};

					if new_dir.norm() > res.norm()
					{
						res = new_dir;
					}
				}
			}
		}
		if res.norm() > 0.
		{
			Some(res)
		}
		else
		{
			None
		}
	}
}

struct Map
{
	world: hecs::World,
	player: hecs::Entity,
	chunks: Vec<MapChunk>,
	camera_pos: comps::Position,
	camera_lookahead: Vector2<f32>,
	show_depth: bool,
}

impl Map
{
	fn new(_state: &mut game_state::GameState) -> Result<Self>
	{
		let mut world = hecs::World::new();

		let spawn_pos = Point3::new(0., 0., 0.);

		let player = spawn_player(spawn_pos, &mut world)?;

		let crystal = spawn_crystal(
			Point3::new(300., 300., 0.),
			comps::ItemKind::Red,
			&mut world,
		)?;
		spawn_from_crystal(crystal, &mut world)?;
		let crystal = spawn_crystal(
			Point3::new(200., 200., 0.),
			comps::ItemKind::Green,
			&mut world,
		)?;
		spawn_from_crystal(crystal, &mut world)?;
		let crystal = spawn_crystal(
			Point3::new(300., 200., 0.),
			comps::ItemKind::Blue,
			&mut world,
		)?;
		spawn_from_crystal(crystal, &mut world)?;

		let crystal = spawn_crystal(
			Point3::new(300., 400., 0.),
			comps::ItemKind::Blue,
			&mut world,
		)?;
		spawn_from_crystal(crystal, &mut world)?;

		Ok(Self {
			world: world,
			player: player,
			chunks: vec![MapChunk::new("data/test.tmx")?],
			camera_pos: comps::Position::new(spawn_pos),
			camera_lookahead: Vector2::zeros(),
			show_depth: false,
		})
	}

	fn camera_to_world(&self, pos: Point2<f32>, state: &game_state::GameState) -> Point2<f32>
	{
		self.camera_pos.pos.xy() + pos.coords
			- Vector2::new(state.buffer_width() / 2., state.buffer_height() / 2.)
	}

	fn camera_shift(&self, state: &game_state::GameState) -> Vector2<f32>
	{
		self.camera_lookahead - self.camera_pos.draw_pos(state.alpha).xy().coords
			+ Vector2::new(state.buffer_width() / 2., state.buffer_height() / 2.)
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
		self.camera_pos.snapshot();

		// Stats.
		for (_, (stats, attack)) in self
			.world
			.query::<(&mut comps::Stats, &comps::Attack)>()
			.iter()
		{
			stats.attacking = attack.want_attack;
		}
		for (_, stats) in self.world.query::<&mut comps::Stats>().iter()
		{
			stats.reset();
		}

		// Input.
		if let Ok(controller) = self
			.world
			.query_one_mut::<&mut comps::Controller>(self.player)
		{
			controller.want_attack = state
				.controls
				.get_action_state(controls::Action::BladeBlade)
				> 0.5;

			let dx = state.controls.get_action_state(controls::Action::MoveRight)
				- state.controls.get_action_state(controls::Action::MoveLeft);
			let dy = state.controls.get_action_state(controls::Action::MoveDown)
				- state.controls.get_action_state(controls::Action::MoveUp);
			controller.want_move = Vector2::new(dx, dy);
			controller.want_jump = state.controls.get_action_state(controls::Action::Jump) > 0.5;
		}

		// AI
		let mut rng = thread_rng();
		for (_, (position, ai, controller)) in self
			.world
			.query::<(&mut comps::Position, &mut comps::AI, &mut comps::Controller)>()
			.iter()
		{
			let idle_time = 3.;
			let wander_time = 0.5;
			let chase_time = 1.;
			let attack_time = 1.;
			let sense_range = 104.;
			let attack_range = 96.;

			// TODO: Better target acquisition.
			let mut target = None;
			if let Some(cur_target) = ai.state.get_target()
			{
				target = Some(cur_target);
			}
			else
			{
				let target_position = self.world.get::<&comps::Position>(self.player).ok();
				if let Some(target_position) = target_position
				{
					let dist = (target_position.pos - position.pos).norm();
					if dist < sense_range && rng.gen_bool(0.9)
					{
						target = Some(self.player);
					}
				}
			}

			if let Some(cur_target) = target
			{
				if !self.world.contains(cur_target)
				{
					target = None;
				}
			}

			let target_position =
				target.and_then(|target| self.world.get::<&comps::Position>(target).ok());
			let mut in_range = false;
			if let Some(target_position) = target_position.as_ref()
			{
				let dist = (target_position.pos - position.pos).norm();
				if dist > 2. * sense_range
				{
					target = None;
				}
				if dist < attack_range
				{
					in_range = true;
				}
			}

			let mut next_state = None;
			match ai.state
			{
				comps::AIState::Idle =>
				{
					controller.want_attack = false;
					if let Some(target) = target
					{
						next_state = Some(comps::AIState::Chase(target));
					}
					else
					{
						controller.want_move = Vector2::zeros();
					}
					if state.time() > ai.next_state_time
					{
						let next_state_weight =
							[(comps::AIState::Idle, 1), (comps::AIState::Wander, 1)];
						next_state = next_state_weight
							.choose_weighted(&mut rng, |sw| sw.1)
							.ok()
							.map(|sw| sw.0);
						match next_state
						{
							Some(comps::AIState::Wander) =>
							{
								let dir_x = rng.gen_range(-1..=1) as f32;
								let dir_y = rng.gen_range(-1..=1) as f32;
								controller.want_move = Vector2::new(dir_x, dir_y);
							}
							_ => (),
						}
					}
				}
				comps::AIState::Wander =>
				{
					controller.want_attack = false;
					if let Some(target) = target
					{
						next_state = Some(comps::AIState::Chase(target));
					}
					if state.time() > ai.next_state_time
					{
						next_state = Some(comps::AIState::Idle);
					}
				}
				comps::AIState::Chase(cur_target) =>
				{
					controller.want_attack = false;
					if let Some(target_position) = target_position
					{
						if in_range
						{
							next_state = Some(comps::AIState::Attack(cur_target));
						}
						else
						{
							let diff = (target_position.pos.xy() - position.pos.xy()).normalize();
							controller.want_move = diff;
						}
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
				comps::AIState::Attack(cur_target) =>
				{
					if let Some(target_position) = target_position
					{
						if in_range
						{
							controller.want_move = Vector2::zeros();
							controller.want_attack = true;
							controller.target_position = target_position.pos;
							let diff = target_position.pos - position.pos;
							position.dir = diff.y.atan2(diff.x);
						}
						else
						{
							next_state = Some(comps::AIState::Chase(cur_target));
						}
					}
					else
					{
						next_state = Some(comps::AIState::Idle);
					}
				}
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

		// Controller.
		for (_, (position, acceleration, stats, controller)) in self
			.world
			.query::<(
				&comps::Position,
				&mut comps::Acceleration,
				&comps::Stats,
				&comps::Controller,
			)>()
			.iter()
		{
			let want_move = controller.want_move;
			let mut air_control = 0.5;
			if position.pos.z == 0.
			{
				air_control = 1.;
			}
			acceleration.pos = Vector3::new(want_move.x, want_move.y, 0.)
				* air_control
				* stats.values.acceleration;
		}

		for (_, (position, velocity, stats, jump, affected_by_gravity, controller)) in self
			.world
			.query::<(
				&comps::Position,
				&mut comps::Velocity,
				&comps::Stats,
				&mut comps::Jump,
				&mut comps::AffectedByGravity,
				&comps::Controller,
			)>()
			.iter()
		{
			let want_jump = controller.want_jump;
			if position.pos.z == 0. && want_jump
			{
				//self.show_depth = !self.show_depth;
				jump.jump_time = state.time();
				velocity.pos.z += stats.values.jump_strength;
			}
			if want_jump && state.time() - jump.jump_time < 0.25
			{
				affected_by_gravity.factor = 0.05;
			}
			else
			{
				affected_by_gravity.factor = 1.;
			}
		}
		for (_, (attack, controller)) in self
			.world
			.query::<(&mut comps::Attack, &comps::Controller)>()
			.iter()
		{
			if controller.want_attack
			{
				attack.want_attack = true;
				attack.target_position = controller.target_position;
			}
		}

		// Appearance animation state handling.
		for (_, (appearance, position, acceleration, velocity)) in self
			.world
			.query::<(
				&mut comps::Appearance,
				&comps::Position,
				&comps::Acceleration,
				&comps::Velocity,
			)>()
			.iter()
		{
			if acceleration.pos.norm() > 0.
			{
				appearance
					.animation_state
					.set_new_animation(format!("Move{}", vec_to_dir_name(acceleration.pos.xy())));
				appearance.speed = velocity.pos.norm() / 196.;
			}
			else
			{
				let dir = Vector2::new(position.dir.cos(), position.dir.sin());
				appearance
					.animation_state
					.set_new_animation(format!("Stand{}", vec_to_dir_name(dir)));
				appearance.speed = 1.;
			}
		}
		for (_, (appearance, position, velocity, _affected_by_gravity)) in self
			.world
			.query::<(
				&mut comps::Appearance,
				&comps::Position,
				&comps::Velocity,
				&comps::AffectedByGravity,
			)>()
			.iter()
		{
			if position.pos.z > 0.
			{
				let dir = if velocity.pos.xy().norm() > 0.
				{
					velocity.pos.xy()
				}
				else
				{
					Vector2::new(position.dir.cos(), position.dir.sin())
				};
				if velocity.pos.z > 0.
				{
					appearance
						.animation_state
						.set_new_animation(format!("Jump{}", vec_to_dir_name(dir)));
				}
				else
				{
					appearance
						.animation_state
						.set_new_animation(format!("Fall{}", vec_to_dir_name(dir)));
				}
				appearance.speed = velocity.pos.z.abs() / 196.;
			}
		}
		for (_, (appearance, position, attack)) in self
			.world
			.query::<(&mut comps::Appearance, &comps::Position, &comps::Attack)>()
			.iter()
		{
			if attack.want_attack
			{
				let dir = Vector2::new(position.dir.cos(), position.dir.sin());
				appearance
					.animation_state
					.set_new_animation(format!("Attack{}", vec_to_dir_name(dir)));
				appearance.speed = 1.; // TODO: cast speed
			}
		}
		for (_, (appearance, _)) in self
			.world
			.query::<(&mut comps::Appearance, &comps::Corpse)>()
			.iter()
		{
			appearance.animation_state.set_new_animation("Dead");
			appearance.speed = 1.;
		}
		for (_, appearance) in self.world.query::<&mut comps::Appearance>().iter()
		{
			let sprite = state.get_sprite(&appearance.sprite)?;
			sprite.advance_state(
				&mut appearance.animation_state,
				(appearance.speed * DT) as f64,
			);
		}

		// Attacking.
		let mut spawn_fns: Vec<Box<dyn FnOnce(&mut hecs::World) -> Result<hecs::Entity>>> = vec![];
		let mut blade_blade_activations = vec![];
		for (id, (appearance, position, attack, stats)) in self
			.world
			.query::<(
				&mut comps::Appearance,
				&comps::Position,
				&mut comps::Attack,
				&comps::Stats,
			)>()
			.iter()
		{
			if attack.want_attack
			{
				let dir = (attack.target_position - position.pos).normalize();
				for _ in 0..appearance.animation_state.drain_activations()
				{
					match attack.kind
					{
						comps::AttackKind::Fireball =>
						{
							// TODO: Spawn position?
							let pos = position.pos.clone();
							let time = state.time();
							spawn_fns.push(Box::new(move |world: &mut hecs::World| {
								spawn_fireball(
									pos + Vector3::new(0., 0., 16.),
									dir * 100.,
									dir * 100.,
									time,
									world,
								)
							}));
						}
						comps::AttackKind::BladeBlade =>
						{
							blade_blade_activations.push((id, stats.values.skill_duration));
						}
					}
				}
				if appearance.animation_state.drain_loops() > 0
				{
					attack.want_attack = false;
				}
			}
		}
		for (id, skill_duration) in blade_blade_activations
		{
			if let Ok(blade_blade) = self.world.query_one_mut::<&mut comps::BladeBlade>(id)
			{
				blade_blade.time_to_remove = state.time() + skill_duration as f64;
				blade_blade.num_blades = utils::min(10, blade_blade.num_blades + 1);
			}
		}

		// Die on activation.
		for (id, (_, appearance)) in self
			.world
			.query::<(&comps::DieOnActivation, &mut comps::Appearance)>()
			.iter()
		{
			if appearance.animation_state.drain_activations() > 0
			{
				to_die.push((true, id));
			}
		}

		// Gravity.
		for (_, (position, acceleration, affected_by_gravity)) in self
			.world
			.query::<(
				&comps::Position,
				&mut comps::Acceleration,
				&comps::AffectedByGravity,
			)>()
			.iter()
		{
			if position.pos.z <= 0.
				&& self.chunks[0].get_tile_kind(position.pos.xy()) == TileKind::Floor
			{
				continue;
			}
			acceleration.pos.z = -affected_by_gravity.factor * 512.;
		}

		// Velocity.
		for (_, (position, acceleration, velocity)) in self
			.world
			.query::<(
				&comps::Position,
				&mut comps::Acceleration,
				&mut comps::Velocity,
			)>()
			.iter()
		{
			if position.pos.z > 0.
			{
				continue;
			}
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

		// Acceleration.
		for (_, (velocity, acceleration, stats)) in self
			.world
			.query::<(&mut comps::Velocity, &comps::Acceleration, &comps::Stats)>()
			.iter()
		{
			velocity.pos = velocity.pos + DT * acceleration.pos;
			if acceleration.pos.xy().norm() > 0.
			{
				let projected_speed = velocity.pos.xy().dot(&acceleration.pos.xy().normalize());
				if projected_speed > stats.values.speed
				{
					velocity
						.pos
						.set_xy(velocity.pos.xy() * stats.values.speed / projected_speed);
				}
			}
		}

		// Position.
		for (_id, (position, velocity)) in self
			.world
			.query::<(&mut comps::Position, &mut comps::Velocity)>()
			.iter()
		{
			position.pos += DT * velocity.pos;
			if velocity.pos.xy().norm() > 0.
			{
				position.dir = velocity.pos.y.atan2(velocity.pos.x);
			}
		}

		// PlaceToDie
		for (id, (position, place_to_die)) in self
			.world
			.query::<(&comps::Position, &mut comps::PlaceToDie)>()
			.iter()
		{
			let dist = (position.pos - place_to_die.target).norm();
			if dist > place_to_die.old_dist
			{
				to_die.push((true, id));
			}
			place_to_die.old_dist = dist;
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
			a_solid.kind.collides_with(b_solid.kind)
		})
		{
			colliding_pairs.push((a.inner, b.inner));
		}

		let mut effects = vec![];
		for pass in 0..5
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
				let diff_z = pos2.z - pos1.z;

				if diff_norm > solid1.size + solid2.size
				{
					continue;
				}
				if diff_z.abs() > solid1.size + solid2.size
				{
					continue;
				}

				if solid1.kind.interacts() && solid2.kind.interacts()
				{
					let diff = 0.9 * diff * (solid1.size + solid2.size - diff_norm) / diff_norm;

					let f1 = 1. - solid1.mass / (solid2.mass + solid1.mass);
					let f2 = 1. - solid2.mass / (solid2.mass + solid1.mass);
					if f32::is_finite(f1)
					{
						let mut position = self.world.get::<&mut comps::Position>(id1)?;
						position.pos.add_xy(-diff * f1);
					}
					if f32::is_finite(f2)
					{
						let mut position = self.world.get::<&mut comps::Position>(id2)?;
						position.pos.add_xy(diff * f2);
					}
				}
				if pass == 0
				{
					for (id, other_id) in [(id1, Some(id2)), (id2, Some(id1))]
					{
						// TODO: Remove this .get.
						if let Ok(on_contact_effect) = self.world.get::<&comps::OnContactEffect>(id)
						{
							effects.push((id, other_id, on_contact_effect.effects.clone()));
						}
					}
				}
			}
			// Floor collision.
			for (id, (position, velocity, solid)) in self
				.world
				.query::<(&mut comps::Position, &mut comps::Velocity, &comps::Solid)>()
				.iter()
			{
				if solid.kind.avoid_holes()
				{
					let push_dir = self.chunks[0].get_escape_dir(
						position.pos.xy(),
						solid.size,
						TileKind::Empty,
					);
					if let Some(push_dir) = push_dir
					{
						position.pos.add_xy(push_dir);
					}
				}
				if position.pos.z < 0.
				{
					if self.chunks[0].get_tile_kind(position.pos.xy()) == TileKind::Floor
					{
						position.pos.z = 0.;
						velocity.pos.z = 0.;
					}
					else
					{
						let push_dir = self.chunks[0].get_escape_dir(
							position.pos.xy(),
							solid.size,
							TileKind::Floor,
						);
						if let Some(push_dir) = push_dir
						{
							position.pos.add_xy(push_dir);
						}
					}
					if let Ok(on_contact_effect) = self.world.get::<&comps::OnContactEffect>(id)
					{
						effects.push((id, None, on_contact_effect.effects.clone()));
					}
				}
			}
		}

		// BladeBlade
		for (id, (position, blade_blade, stats)) in self
			.world
			.query::<(&comps::Position, &mut comps::BladeBlade, &comps::Stats)>()
			.iter()
		{
			if state.time() > blade_blade.time_to_remove
			{
				blade_blade.num_blades = utils::max(0, blade_blade.num_blades - 1);
				blade_blade.time_to_remove = state.time() + stats.values.skill_duration as f64;
			}
			if state.time() > blade_blade.time_to_hit && blade_blade.num_blades > 0
			{
				blade_blade.time_to_hit = state.time() + 0.5 / blade_blade.num_blades as f64;

				let r = stats.values.area_of_effect.sqrt();
				let rv = Vector2::new(r, r);
				let entries =
					grid.query_rect(position.pos.xy() - rv, position.pos.xy() + rv, |other| {
						let other_id = other.inner.id;
						if id == other_id
						{
							false
						}
						else if let Some(other_stats) = self
							.world
							.query_one::<&comps::Stats>(other_id)
							.unwrap()
							.get()
						{
							other_stats.values.team != stats.values.team
						}
						else
						{
							false
						}
					});
				for entry in entries
				{
					let other_id = entry.inner.id;
					if let Some(other_position) = self
						.world
						.query_one::<&comps::Position>(other_id)
						.unwrap()
						.get()
					{
						let diff_xy = position.pos.xy() - other_position.pos.xy();
						let diff_z = position.pos.z - other_position.pos.z;
						if diff_xy.norm() < r && diff_z.abs() < 16.
						{
							effects.push((
								id,
								Some(other_id),
								vec![
									comps::Effect::SpawnFireHit,
									comps::Effect::DoDamage(
										comps::Damage::Magic(1.),
										stats.values.team,
									),
								],
							));
						}
					}
				}
			}
		}

		// Crystal
		for (id, crystal) in self.world.query::<&comps::Crystal>().iter()
		{
			if crystal.enemies <= 0
			{
				to_die.push((true, id));
			}
		}

		// Camera
		if let Ok(position) = self.world.get::<&comps::Position>(self.player)
		{
			self.camera_pos.pos += 0.25 * (position.pos - self.camera_pos.pos);
			// TODO: Think about this.
			self.camera_lookahead = -0. * (position.pos.xy() - self.camera_pos.pos.xy());
		}

		// Time to die
		for (id, time_to_die) in self.world.query_mut::<&comps::TimeToDie>()
		{
			if state.time() > time_to_die.time
			{
				to_die.push((true, id));
			}
		}

		// Die on zero health.
		for (id, stats) in self.world.query::<&comps::Stats>().iter()
		{
			if stats.values.health <= 0.
			{
				to_die.push((true, id));
			}
		}

		// On death effects.
		for &(_, id) in to_die.iter().filter(|(on_death, _)| *on_death)
		{
			if let Ok(on_death_effects) = self.world.get::<&comps::OnDeathEffect>(id)
			{
				effects.push((id, None, on_death_effects.effects.clone()));
			}
		}

		// Effects.
		for (id, other_id, effects) in effects
		{
			for effect in effects
			{
				match (effect, other_id)
				{
					(comps::Effect::Die, _) => to_die.push((false, id)),
					(comps::Effect::SpawnFireHit, other_id) =>
					{
						let mut pos = None;
						if let Some(position) = other_id
							.and_then(|other_id| self.world.get::<&comps::Position>(other_id).ok())
						{
							pos = Some(position.pos.clone() + Vector3::new(0., 0., 16.));
						}
						else if let Ok(position) = self.world.get::<&comps::Position>(id)
						{
							pos = Some(position.pos.clone());
						}

						if let Some(pos) = pos
						{
							spawn_fns.push(Box::new(move |world: &mut hecs::World| {
								spawn_fire_hit(pos, world)
							}));
						}
					}
					(comps::Effect::DoDamage(damage, team), Some(other_id)) =>
					{
						if let Ok(stats) = self.world.query_one_mut::<&mut comps::Stats>(other_id)
						{
							if stats.values.team != team
							{
								stats.apply_damage(damage);
							}
						}
					}
					(comps::Effect::SpawnCorpse, _) =>
					{
						if let Ok((position, appearance, velocity)) =
							self.world
								.query_one_mut::<(&comps::Position, &comps::Appearance, &comps::Velocity)>(
									id,
								)
						{
							let pos = position.pos.clone();
							let vel_pos = velocity.pos.clone();
							let mut appearance = appearance.clone();
							appearance.bias = -1;
							spawn_fns.push(Box::new(move |world: &mut hecs::World| {
								spawn_corpse(pos, vel_pos, appearance, world)
							}));
						}
					}
					(comps::Effect::SpawnSoul(crystal_id), _) =>
					{
						let mut crystal_pos = None;
						if let Ok((position, _)) = self
							.world
							.query_one_mut::<(&comps::Position, &mut comps::Crystal)>(crystal_id)
						{
							crystal_pos = Some(position.pos);
						}
						let mut src_pos = None;
						if let Ok(position) = self.world.query_one_mut::<&comps::Position>(id)
						{
							src_pos = Some(position.pos);
						}
						if let (Some(pos), Some(target)) = (src_pos, crystal_pos)
						{
							spawn_fns.push(Box::new(move |world: &mut hecs::World| {
								spawn_soul(
									pos + Vector3::new(0., 0., 16.),
									target + Vector3::new(0., 0., 16.),
									crystal_id,
									world,
								)
							}));
						}
					}
					(comps::Effect::UnlockCrystal(crystal_id), _) =>
					{
						if let Ok(crystal) =
							self.world.query_one_mut::<&mut comps::Crystal>(crystal_id)
						{
							crystal.enemies -= 1;
						}
					}
					(comps::Effect::SpawnPowerSphere(kind), _) =>
					{
						let mut sphere_spawns = vec![];
						let mut src_pos = None;
						if let Ok(position) = self.world.query_one_mut::<&comps::Position>(id)
						{
							src_pos = Some(position.pos);
						}
						for (id, (position, crystal)) in self
							.world
							.query::<(&mut comps::Position, &mut comps::Crystal)>()
							.iter()
						{
							if crystal.kind != kind
							{
								sphere_spawns.push((id, position.pos));
							}
						}
						if let Some(pos) = src_pos
						{
							for (id, target) in sphere_spawns
							{
								spawn_fns.push(Box::new(move |world: &mut hecs::World| {
									spawn_power_sphere(
										pos + Vector3::new(0., 0., 16.),
										target + Vector3::new(0., 0., 16.),
										id,
										world,
									)
								}));
							}
						}
					}
					(comps::Effect::ElevateCrystal(crystal_id), _) =>
					{
						if let Ok(crystal) =
							self.world.query_one_mut::<&mut comps::Crystal>(crystal_id)
						{
							crystal.level += 1;
						}
						spawn_from_crystal(crystal_id, &mut self.world)?;
					}
					_ => panic!(),
				}
			}
		}

		for spawn_fn in spawn_fns
		{
			spawn_fn(&mut self.world)?;
		}

		// Remove dead entities
		to_die.sort_by_key(|s_id| s_id.1);
		to_die.dedup_by_key(|s_id| s_id.1);
		for (_, id) in to_die
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
		state.core.clear_depth_buffer(-1.);
		state.core.set_depth_test(Some(DepthFunction::Greater));
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

		let camera_shift = &self.camera_shift(state);

		// Tiles and appearances
		let mut scene = Scene::new();
		state
			.core
			.use_shader(Some(&*state.palette_shader.upgrade().unwrap()))
			.unwrap();

		for chunk in self.chunks.iter_mut()
		{
			chunk.draw(
				Point2::new(camera_shift.x, camera_shift.y),
				&mut scene,
				-0.7 * TILE_SIZE - self.camera_pos.pos.y,
				state,
			)?;
		}
		for (_, (appearance, position)) in self
			.world
			.query_mut::<(&comps::Appearance, &comps::Position)>()
		{
			let sprite = state.get_sprite(&appearance.sprite)?;
			let palette_index = state.palettes.get_palette_index(
				appearance
					.palette
					.as_ref()
					.unwrap_or(&sprite.get_palettes()[0]),
			)?;

			state.core.set_shader_uniform("use_texture", &[1][..]).ok();
			state
				.core
				.set_shader_uniform("show_depth", &[self.show_depth as i32 as f32][..])
				.ok();
			state
				.core
				.set_shader_sampler("palette", &state.palettes.palette_bitmap, 2)
				.ok();

			let draw_pos = position.draw_pos(state.alpha);
			let pos = utils::round_point(
				utils::round_point(Point2::new(draw_pos.x, draw_pos.y - draw_pos.z)) + camera_shift,
			);

			let (atlas_bmp, offt) = sprite.get_frame_from_state(&appearance.animation_state);

			scene.add_bitmap(
				Point3::new(
					pos.x + offt.x,
					pos.y + offt.y,
					position.pos.y - self.camera_pos.pos.y + appearance.bias as f32,
				),
				atlas_bmp,
				palette_index,
				0,
			);
		}

		// Crystal pips.
		for (_, (position, crystal)) in self
			.world
			.query_mut::<(&comps::Position, &comps::Crystal)>()
		{
			let sprite = state.get_sprite("data/crystal_pips.cfg")?;
			let palette_index = state
				.palettes
				.get_palette_index(&sprite.get_palettes()[0])?;

			let draw_pos = position.draw_pos(state.alpha);
			let pos = utils::round_point(
				utils::round_point(Point2::new(draw_pos.x, draw_pos.y - draw_pos.z)) + camera_shift,
			);
			let (atlas_bmp, offt) = sprite.get_frame("Default", crystal.level);

			scene.add_bitmap(
				Point3::new(
					pos.x + offt.x,
					pos.y + offt.y,
					position.pos.y - self.camera_pos.pos.y + 1.,
				),
				atlas_bmp,
				palette_index,
				0,
			);
		}

		// Shadows.
		for (_, (position, _)) in self
			.world
			.query_mut::<(&comps::Position, &comps::CastsShadow)>()
		{
			if self.chunks[0].get_tile_kind(position.pos.xy()) != TileKind::Floor
			{
				continue;
			}
			let sprite = state.get_sprite("data/shadow.cfg")?;
			let palette_index = state
				.palettes
				.get_palette_index(&sprite.get_palettes()[0])?;

			let draw_pos = position.draw_pos(state.alpha);
			let pos = utils::round_point(
				utils::round_point(Point2::new(draw_pos.x, draw_pos.y)) + camera_shift,
			);
			let (atlas_bmp, offt) = sprite.get_frame("Default", 0);

			scene.add_bitmap(
				Point3::new(
					pos.x + offt.x,
					pos.y + offt.y,
					position.pos.y - self.camera_pos.pos.y - 2.,
				),
				atlas_bmp,
				palette_index,
				0,
			);
		}

		for (i, page) in state.atlas.pages.iter().enumerate()
		{
			state.prim.draw_indexed_prim(
				&scene.buckets[i].vertices[..],
				Some(&page.bitmap),
				&scene.buckets[i].indices[..],
				0,
				scene.buckets[i].indices.len() as u32,
				PrimType::TriangleList,
			);
		}

		state
			.core
			.use_shader(Some(&*state.basic_shader.upgrade().unwrap()))
			.unwrap();

		// BladeBlade
		let mut trail_vertices = vec![];
		let mut blade_vertices = vec![];
		let mut blade_indices = vec![];

		for (_, (position, blade_blade, stats)) in self
			.world
			.query::<(&comps::Position, &comps::BladeBlade, &comps::Stats)>()
			.iter()
		{
			let draw_pos = position.draw_pos(state.alpha);
			let pos = utils::round_point(
				Point2::new(draw_pos.x, draw_pos.y - draw_pos.z - 8.) + camera_shift,
			);

			let radii: Vec<_> = [1., 0.5, 0.3, 0.7, 0.1, 0.6, 0.4, 0.9, 0.2, 0.8]
				.iter()
				.map(|&r| (r - 0.1) / 0.9 * 0.8 + 0.2)
				.collect();
			let offsets = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.];
			let speeds = [0.1, 0.3, 0.5, 0.7, 1.1, 1.3, 1.7, 1.9, 2.3, 3.1];
			let color = Color::from_rgb_f(1., 0.2, 0.2);

			for blade in 0..blade_blade.num_blades
			{
				let r = stats.values.area_of_effect.sqrt() * radii[blade as usize];

				let theta = 2.
					* std::f64::consts::PI
					* (state.time() / (1. - 0.5 * speeds[blade as usize] / 3.)
						+ offsets[blade as usize]);
				let theta = theta.rem_euclid(2. * std::f64::consts::PI) as f32;

				let one_blade_vertices = [
					Point2::new(0.5f32, 0.),
					Point2::new(0., 1.0),
					Point2::new(0., 0.),
					Point2::new(0., -1.0),
				];

				let rot = Rotation2::new(theta);
				let idx = blade_vertices.len() as i32;
				blade_indices.extend([idx + 0, idx + 1, idx + 3, idx + 1, idx + 2, idx + 3]);
				for vtx in one_blade_vertices
				{
					let vtx = rot * (3. * vtx + Vector2::new(r, 0.));
					let z = position.pos.y + vtx.y - self.camera_pos.pos.y;
					blade_vertices.push(Vertex {
						x: pos.x + vtx.x,
						y: pos.y + vtx.y,
						z: z,
						u: 0.,
						v: 0.,
						color: color,
					});
				}

				for i in 0..10
				{
					for j in 0..2
					{
						let theta2 = -0.25 * PI * (i + j) as f32 / 10.;
						let dx = r * (theta2 + theta).cos();
						let dy = r * (theta2 + theta).sin();
						let z = position.pos.y + dy - self.camera_pos.pos.y;

						trail_vertices.push(Vertex {
							x: pos.x + dx,
							y: pos.y + dy,
							z: z,
							u: 0.,
							v: 0.,
							color: color,
						})
					}
				}
			}
		}
		state.prim.draw_prim(
			&trail_vertices[..],
			Option::<&Bitmap>::None,
			0,
			trail_vertices.len() as u32,
			PrimType::LineList,
		);
		state.prim.draw_indexed_prim(
			&blade_vertices[..],
			Option::<&Bitmap>::None,
			&blade_indices[..],
			0,
			blade_indices.len() as u32,
			PrimType::TriangleList,
		);

		//for (_, position) in &appearances
		//{
		//	let draw_pos = position.draw_pos(state.alpha);
		//	let pos =
		//		utils::round_point(Point2::new(draw_pos.x, draw_pos.y - draw_pos.z) + camera_shift);

		//	for i in 0..10
		//	{
		//	state.prim.draw_elliptical_arc(
		//		pos.x,
		//		pos.y - 16.,
		//		16. + i as f32 * 2.,
		//		8.  + i as f32 * 1.,
		//		(117. * i as f64 + 8. * state.time() % (2. * std::f64::consts::PI)) as f32,
		//		std::f32::consts::PI / 4.,
		//		Color::from_rgb_f(1., 0., 0.),
		//		-1.,
		//	);
		//	}
		//}

		Ok(())
	}
}
