use crate::error::Result;
use crate::sprite;
use allegro::*;
use na::{Point3, Vector2, Vector3};
use nalgebra as na;
use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone)]
pub struct Position
{
	pub pos: Point3<f32>,
	pub dir: f32,
	old_pos: Point3<f32>,
}

impl Position
{
	pub fn new(pos: Point3<f32>) -> Self
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

	pub fn draw_pos(&self, alpha: f32) -> Point3<f32>
	{
		self.pos + alpha * (self.pos - self.old_pos)
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Velocity
{
	pub pos: Vector3<f32>,
}

#[derive(Debug, Copy, Clone)]
pub struct Acceleration
{
	pub pos: Vector3<f32>,
}

#[derive(Debug, Clone)]
pub struct Appearance
{
	pub sprite: String,
	pub palette: Option<String>,
	pub animation_state: sprite::AnimationState,
	pub speed: f32,
	pub bias: i32,
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
			bias: 0,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum CollisionKind
{
	BigEnemy,
	BigPlayer,
	SmallEnemy,
	SmallPlayer,
	World,
}

impl CollisionKind
{
	pub fn collides_with(&self, other: Self) -> bool
	{
		match (self, other)
		{
			(CollisionKind::BigEnemy, CollisionKind::BigEnemy) => true,
			(CollisionKind::BigEnemy, CollisionKind::BigPlayer) => true,
			(CollisionKind::BigEnemy, CollisionKind::SmallEnemy) => false,
			(CollisionKind::BigEnemy, CollisionKind::SmallPlayer) => true,
			(CollisionKind::BigEnemy, CollisionKind::World) => true,

			(CollisionKind::BigPlayer, CollisionKind::BigEnemy) => true,
			(CollisionKind::BigPlayer, CollisionKind::BigPlayer) => true,
			(CollisionKind::BigPlayer, CollisionKind::SmallEnemy) => true,
			(CollisionKind::BigPlayer, CollisionKind::SmallPlayer) => false,
			(CollisionKind::BigPlayer, CollisionKind::World) => true,

			(CollisionKind::SmallEnemy, CollisionKind::BigEnemy) => false,
			(CollisionKind::SmallEnemy, CollisionKind::BigPlayer) => true,
			(CollisionKind::SmallEnemy, CollisionKind::SmallEnemy) => false,
			(CollisionKind::SmallEnemy, CollisionKind::SmallPlayer) => false,
			(CollisionKind::SmallEnemy, CollisionKind::World) => true,

			(CollisionKind::SmallPlayer, CollisionKind::BigEnemy) => true,
			(CollisionKind::SmallPlayer, CollisionKind::BigPlayer) => false,
			(CollisionKind::SmallPlayer, CollisionKind::SmallEnemy) => false,
			(CollisionKind::SmallPlayer, CollisionKind::SmallPlayer) => false,
			(CollisionKind::SmallPlayer, CollisionKind::World) => true,

			(CollisionKind::World, CollisionKind::BigEnemy) => true,
			(CollisionKind::World, CollisionKind::BigPlayer) => false,
			(CollisionKind::World, CollisionKind::SmallEnemy) => false,
			(CollisionKind::World, CollisionKind::SmallPlayer) => false,
			(CollisionKind::World, CollisionKind::World) => true,
		}
	}

	pub fn interacts(&self) -> bool
	{
		true
	}

	pub fn avoid_holes(&self) -> bool
	{
		match self
		{
			CollisionKind::BigEnemy => true,
			_ => false,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Solid
{
	pub size: f32,
	pub mass: f32,
	pub kind: CollisionKind,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Team
{
	Player,
	Enemy,
	Neutral,
}

impl Team
{
	pub fn can_damage(&self, other: Team) -> bool
	{
		match (self, other)
		{
			(Team::Player, Team::Enemy) => true,
			(Team::Enemy, Team::Player) => true,
			_ => false,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct StatValues
{
	pub speed: f32,
	pub acceleration: f32,
	pub jump_strength: f32,
	pub area_of_effect: f32,
	pub skill_duration: f32,
	pub health: f32,
	pub team: Team,
}

impl Default for StatValues
{
	fn default() -> Self
	{
		Self {
			speed: 0.,
			acceleration: 0.,
			jump_strength: 0.,
			area_of_effect: 0.,
			skill_duration: 0.,
			health: 10.,
			team: Team::Enemy,
		}
	}
}

impl StatValues
{
	pub fn new_player() -> Self
	{
		Self {
			speed: 196.,
			acceleration: 1024.,
			jump_strength: 128.,
			area_of_effect: 32. * 32.,
			skill_duration: 1.,
			team: Team::Player,
			health: 100.,
			..Self::default()
		}
	}

	pub fn new_enemy() -> Self
	{
		Self {
			speed: 64.,
			acceleration: 1024.,
			skill_duration: 0.25,
			health: 1.,
			..Self::default()
		}
	}

	pub fn new_fireball() -> Self
	{
		Self {
			speed: 256.,
			acceleration: 1024.,
			..Self::default()
		}
	}

	pub fn new_item() -> Self
	{
		Self {
			speed: 256.,
			team: Team::Neutral,
			jump_strength: 128.,
			..Self::default()
		}
	}

	pub fn new_corpse() -> Self
	{
		Self {
			speed: 256.,
			team: Team::Neutral,
			..Self::default()
		}
	}
}

#[derive(Debug, Clone)]
pub struct Stats
{
	pub base_values: StatValues,
	pub values: StatValues,

	pub attacking: bool,
	pub damage: f32,
	pub dead: bool,
}

impl Stats
{
	pub fn new(base_values: StatValues) -> Self
	{
		Self {
			base_values: base_values,
			values: base_values,
			attacking: false,
			damage: 0.,
			dead: false,
		}
	}

	pub fn reset(&mut self)
	{
		self.values = self.base_values;
		if self.attacking
		{
			self.values.acceleration = 0.;
			self.values.jump_strength = 0.;
		}
		if self.dead
		{
			self.values.team = Team::Neutral;
		}
		self.values.health -= self.damage;
	}

	pub fn apply_damage(&mut self, damage: Damage)
	{
		match damage
		{
			Damage::Magic(damage) =>
			{
				self.damage += damage;
			}
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
	pub target_position: Point3<f32>,
	pub kind: AttackKind,
}

impl Attack
{
	pub fn new(kind: AttackKind) -> Self
	{
		Self {
			want_attack: false,
			target_position: Point3::new(0., 0., 0.),
			kind: kind,
		}
	}
}

#[derive(Debug, Copy, Clone)]
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
pub struct PlaceToDie
{
	pub target: Point3<f32>,
	pub old_dist: f32,
}

impl PlaceToDie
{
	pub fn new(target: Point3<f32>) -> Self
	{
		Self {
			target: target,
			old_dist: std::f32::INFINITY,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum Damage
{
	Magic(f32),
}

#[derive(Debug, Copy, Clone)]
pub enum Effect
{
	Die,
	SpawnFireHit,
	DoDamage(Damage, Team),
	SpawnCorpse,
	SpawnSoul(hecs::Entity),
	UnlockCrystal(hecs::Entity),
	SpawnPowerSphere(ItemKind),
	ElevateCrystal(hecs::Entity),
	SpawnItems(ItemKind),
}

#[derive(Debug, Clone)]
pub struct OnContactEffect
{
	pub effects: Vec<Effect>,
}

#[derive(Debug, Clone)]
pub struct OnDeathEffect
{
	pub effects: Vec<Effect>,
}

#[derive(Debug, Copy, Clone)]
pub struct AffectedByGravity
{
	pub factor: f32,
}

impl AffectedByGravity
{
	pub fn new() -> Self
	{
		Self { factor: 1. }
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Jump
{
	pub jump_time: f64,
}

impl Jump
{
	pub fn new() -> Self
	{
		Self { jump_time: 0. }
	}
}

pub struct DieOnActivation;

#[derive(Debug, Copy, Clone)]
pub struct BladeBlade
{
	pub num_blades: i32,
	pub time_to_remove: f64,
	pub time_to_hit: f64,
}

impl BladeBlade
{
	pub fn new() -> Self
	{
		Self {
			num_blades: 0,
			time_to_remove: 0.,
			time_to_hit: 0.,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct CastsShadow;

#[derive(Debug, Copy, Clone)]
pub struct Controller
{
	pub want_attack: bool,
	pub want_jump: bool,
	pub want_move: Vector2<f32>,
	pub target_position: Point3<f32>,
}

impl Controller
{
	pub fn new() -> Self
	{
		Self {
			want_attack: false,
			want_jump: false,
			want_move: Vector2::zeros(),
			target_position: Point3::new(0., 0., 0.),
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Corpse;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ItemKind
{
	Blue,
	Red,
	Green,
}

#[derive(Debug, Copy, Clone)]
pub struct Crystal
{
	pub kind: ItemKind,
	pub level: i32,
	pub enemies: i32,
}

impl Crystal
{
	pub fn new(kind: ItemKind) -> Self
	{
		Self {
			kind: kind,
			level: 0,
			enemies: 0,
		}
	}
}

#[derive(Debug, Clone)]
pub struct Item
{
	pub name: String,
	pub appearance: Appearance,
}

#[derive(Debug, Clone)]
pub struct Inventory
{
	pub slots: [Option<Item>; 6],
}

impl Inventory
{
	pub fn new() -> Self
	{
		Self {
			slots: [None, None, None, None, None, None],
		}
	}
}
