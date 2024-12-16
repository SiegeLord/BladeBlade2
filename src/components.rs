use crate::sprite;
use allegro::*;
use na::{Point2, Vector2};
use nalgebra as na;
use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone)]
pub struct Position
{
	pub pos: Point2<f32>,
	pub dir: f32,
	old_pos: Point2<f32>,
}

impl Position
{
	pub fn new(pos: Point2<f32>) -> Self
	{
		Self {
			pos,
			dir: 0.,
			old_pos: pos,
		}
	}

	pub fn snapshot(&mut self)
	{
		self.old_pos = self.pos;
	}

	pub fn draw_pos(&self, alpha: f32) -> Point2<f32>
	{
		self.pos + alpha * (self.pos - self.old_pos)
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Velocity
{
	pub pos: Vector2<f32>,
}

#[derive(Debug, Copy, Clone)]
pub struct Acceleration
{
	pub pos: Vector2<f32>,
}

#[derive(Debug, Clone)]
pub struct Appearance
{
	pub sprite: String,
	pub palette: Option<String>,
	pub animation_state: sprite::AnimationState,
	pub speed: f32,
}

impl Appearance
{
	pub fn new(sprite: impl Into<String>) -> Self
	{
		Self {
			sprite: sprite.into(),
			palette: None,
			animation_state: sprite::AnimationState::new("Default"),
			speed: 1.,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum CollisionClass
{
	BigEnemy,
	BigPlayer,
	SmallEnemy,
	SmallPlayer,
}

impl CollisionClass
{
	pub fn collides_with(&self, other: Self) -> bool
	{
		match (self, other)
		{
			(CollisionClass::BigEnemy, CollisionClass::BigEnemy) => true,
			(CollisionClass::BigEnemy, CollisionClass::BigPlayer) => true,
			(CollisionClass::BigEnemy, CollisionClass::SmallEnemy) => false,
			(CollisionClass::BigEnemy, CollisionClass::SmallPlayer) => true,

			(CollisionClass::BigPlayer, CollisionClass::BigEnemy) => true,
			(CollisionClass::BigPlayer, CollisionClass::BigPlayer) => true,
			(CollisionClass::BigPlayer, CollisionClass::SmallEnemy) => true,
			(CollisionClass::BigPlayer, CollisionClass::SmallPlayer) => false,

			(CollisionClass::SmallEnemy, CollisionClass::BigEnemy) => false,
			(CollisionClass::SmallEnemy, CollisionClass::BigPlayer) => true,
			(CollisionClass::SmallEnemy, CollisionClass::SmallEnemy) => false,
			(CollisionClass::SmallEnemy, CollisionClass::SmallPlayer) => false,

			(CollisionClass::SmallPlayer, CollisionClass::BigEnemy) => true,
			(CollisionClass::SmallPlayer, CollisionClass::BigPlayer) => false,
			(CollisionClass::SmallPlayer, CollisionClass::SmallEnemy) => false,
			(CollisionClass::SmallPlayer, CollisionClass::SmallPlayer) => false,
		}
	}
	pub fn interacts(&self) -> bool
	{
		true
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Solid
{
	pub size: f32,
	pub mass: f32,
	pub collision_class: CollisionClass,
}

#[derive(Debug, Copy, Clone)]
pub enum AIState
{
	Idle,
	Wander,
	Chase(hecs::Entity),
	Attack(hecs::Entity),
}

impl AIState
{
	pub fn get_target(&self) -> Option<hecs::Entity>
	{
		match self
		{
			AIState::Chase(e) => Some(*e),
			_ => None,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct AI
{
	pub state: AIState,
	pub next_state_time: f64,
	pub target: Option<hecs::Entity>,
}

impl AI
{
	pub fn new() -> Self
	{
		Self {
			state: AIState::Idle,
			next_state_time: 0.,
			target: None,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct StatValues
{
	pub speed: f32,
	pub acceleration: f32,
}

impl StatValues
{
	pub fn new_player() -> Self
	{
		Self {
			speed: 196.,
			acceleration: 1024.,
		}
	}

	pub fn new_enemy() -> Self
	{
		Self {
			speed: 64.,
			acceleration: 1024.,
		}
	}

	pub fn new_fireball() -> Self
	{
		Self {
			speed: 256.,
			acceleration: 1024.,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Stats
{
	pub base_values: StatValues,
	pub values: StatValues,
}

impl Stats
{
	pub fn new(base_values: StatValues) -> Self
	{
		Self {
			base_values: base_values,
			values: base_values,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum AttackKind
{
	BladeBlade,
	Fireball,
}

#[derive(Debug, Copy, Clone)]
pub struct Attack
{
	pub want_attack: bool,
	pub target_position: Point2<f32>,
	pub kind: AttackKind,
}

impl Attack
{
	pub fn new(kind: AttackKind) -> Self
	{
		Self {
			want_attack: false,
			target_position: Point2::new(0., 0.),
			kind: kind,
		}
	}
}

pub struct TimeToDie
{
	pub time: f64,
}

impl TimeToDie
{
	pub fn new(time: f64) -> Self
	{
		Self { time: time }
	}
}

#[derive(Debug, Copy, Clone)]
pub enum ContactEffect
{
	Die,
}

#[derive(Debug, Clone)]
pub struct OnContactEffect
{
	pub effects: Vec<ContactEffect>,
}
