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
	pub max_health: f32,
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
			max_health: 10.,
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
			max_health: 100.,
			..Self::default()
		}
	}

	pub fn new_enemy() -> Self
	{
		Self {
			speed: 64.,
			acceleration: 1024.,
			skill_duration: 0.25,
			max_health: 1.,
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
	pub health: f32,
	pub old_max_health: f32,
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
			health: base_values.max_health,
			old_max_health: base_values.max_health,
			dead: false,
		}
	}

	pub fn reset(&mut self, inventory: Option<&Inventory>)
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
	}

	pub fn apply_damage(&mut self, damage: Damage)
	{
		match damage
		{
			Damage::Magic(damage) =>
			{
				self.health -= damage;
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
#[repr(i32)]
pub enum ItemKind
{
	Red = 0,
	Green = 1,
	Blue = 2,
}

impl ItemKind
{
	pub fn to_str(&self) -> &'static str
	{
		match self
		{
			ItemKind::Red => "Ruby Ring",
			ItemKind::Green => "Emerald Ring",
			ItemKind::Blue => "Sapphire Ring",
		}
	}
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum ItemPrefix
{
	Health = 0,
	HealthRegen = 1,
	AddedDamage = 2,
	AddedColdDamage = 3,
	AddedFireDamage = 4,
	AddedLightningDamage = 5,
	CriticalChance = 6,
	ChanceToFreeze = 7,
	ChanceToIgnite = 8,
	ChanceToShock = 9,
}

impl ItemPrefix
{
	pub fn to_str(&self, tier: i32) -> &'static str
	{
		match self
		{
			ItemPrefix::Health => ["Robust", "Healthy", "Jolly"][tier as usize],
			ItemPrefix::HealthRegen => ["Bubbly", "Rolling", "Boiling"][tier as usize],
			ItemPrefix::AddedDamage => ["Rough", "Spiked", "Poky"][tier as usize],
			ItemPrefix::AddedColdDamage => ["Cold", "Snowy", "Icy"][tier as usize],
			ItemPrefix::AddedFireDamage => ["Warm", "Fiery", "Blazing"][tier as usize],
			ItemPrefix::AddedLightningDamage => ["Sparking", "Electric", "Ohm's"][tier as usize],
			ItemPrefix::CriticalChance => ["Pointed", "Sharp", "Vorpal"][tier as usize],
			ItemPrefix::ChanceToFreeze => ["Cooling", "Chilling", "Freezing"][tier as usize],
			ItemPrefix::ChanceToIgnite => ["Igniting", "Burning", "Flaming"][tier as usize],
			ItemPrefix::ChanceToShock => ["Shocking", "Zapping", "Stunning"][tier as usize],
			// TODO Mana, mana regen, leech
		}
	}
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum ItemSuffix
{
	Armour = 0,
	PhysicalResistance = 1,
	ColdResistance = 2,
	FireResistance = 3,
	LightningResistance = 4,
	CriticalMultiplier = 5,
	IncreasedDamage = 6,
	IncreasedColdDamage = 7,
	IncreasedFireDamage = 8,
	IncreasedLightningDamage = 9,
}

impl ItemSuffix
{
	pub fn to_str(&self, tier: i32) -> &'static str
	{
		match self
		{
			ItemSuffix::Armour => ["of Clay", "of Granite", "of Gneiss"][tier as usize],
			ItemSuffix::PhysicalResistance => ["of Mail", "of Scale", "of Plate"][tier as usize],
			ItemSuffix::ColdResistance => ["of the Coat", "of Fur", "of Yeti"][tier as usize],
			ItemSuffix::FireResistance =>
			{
				["of Dousing", "of Antipyre", "of Asbestos"][tier as usize]
			}
			ItemSuffix::LightningResistance =>
			{
				["of Grounding", "of the Rod", "of Graphite"][tier as usize]
			}
			ItemSuffix::CriticalMultiplier =>
			{
				["of Poking", "of Piercing", "of Slicing"][tier as usize]
			}
			ItemSuffix::IncreasedDamage => ["of Spikes", "of Pikes", "of Blades"][tier as usize],
			ItemSuffix::IncreasedColdDamage =>
			{
				["of Penguin", "of Iceberg", "of Ice"][tier as usize]
			}
			ItemSuffix::IncreasedFireDamage =>
			{
				["of Salamander", "of Dragon", "of Inferno"][tier as usize]
			}
			ItemSuffix::IncreasedLightningDamage =>
			{
				["of Zapping", "of Thunder", "of Lightning"][tier as usize]
			}
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum Rarity
{
	Magic,
	Rare,
}

#[derive(Debug, Clone)]
pub struct Item
{
	pub name: Vec<String>,
	pub appearance: Appearance,
	pub prefixes: Vec<(ItemPrefix, i32, f32)>,
	pub suffixes: Vec<(ItemSuffix, i32, f32)>,
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

pub fn generate_item(kind: ItemKind, crystal_level: i32, level: i32, rng: &mut impl Rng) -> Item
{
	let rarity_weights = match crystal_level
	{
		0 => (50, 5),
		1 => (40, 5),
		2 => (30, 5),
		3 => (20, 5),
		4 => (10, 5),
		5 => (5, 10),
		6 => (5, 20),
		7 => (5, 30),
		_ => unreachable!(),
	};

	let rarity = [
		(Rarity::Magic, rarity_weights.0),
		(Rarity::Rare, rarity_weights.1),
	]
	.choose_weighted(rng, |&(_, w)| w)
	.unwrap()
	.0;

	let red_prefix_weights = [
		(ItemPrefix::Health, 1000),
		(ItemPrefix::HealthRegen, 200),
		(ItemPrefix::AddedDamage, 50),
		(ItemPrefix::AddedColdDamage, 50),
		(ItemPrefix::AddedFireDamage, 1000),
		(ItemPrefix::AddedLightningDamage, 50),
		(ItemPrefix::CriticalChance, 50),
		(ItemPrefix::ChanceToFreeze, 10),
		(ItemPrefix::ChanceToIgnite, 50),
		(ItemPrefix::ChanceToShock, 10),
	];

	let green_prefix_weights = [
		(ItemPrefix::Health, 50),
		(ItemPrefix::HealthRegen, 50),
		(ItemPrefix::AddedDamage, 50),
		(ItemPrefix::AddedColdDamage, 50),
		(ItemPrefix::AddedFireDamage, 50),
		(ItemPrefix::AddedLightningDamage, 1000),
		(ItemPrefix::CriticalChance, 500),
		(ItemPrefix::ChanceToFreeze, 10),
		(ItemPrefix::ChanceToIgnite, 10),
		(ItemPrefix::ChanceToShock, 50),
	];

	let blue_prefix_weights = [
		(ItemPrefix::Health, 50),
		(ItemPrefix::HealthRegen, 50),
		(ItemPrefix::AddedDamage, 50),
		(ItemPrefix::AddedColdDamage, 1000),
		(ItemPrefix::AddedFireDamage, 50),
		(ItemPrefix::AddedLightningDamage, 50),
		(ItemPrefix::CriticalChance, 50),
		(ItemPrefix::ChanceToFreeze, 50),
		(ItemPrefix::ChanceToIgnite, 10),
		(ItemPrefix::ChanceToShock, 10),
	];

	let red_suffix_weights = [
		(ItemSuffix::Armour, 50),
		(ItemSuffix::PhysicalResistance, 50),
		(ItemSuffix::ColdResistance, 500),
		(ItemSuffix::FireResistance, 1000),
		(ItemSuffix::LightningResistance, 500),
		(ItemSuffix::CriticalMultiplier, 100),
		(ItemSuffix::IncreasedDamage, 50),
		(ItemSuffix::IncreasedColdDamage, 500),
		(ItemSuffix::IncreasedFireDamage, 1000),
		(ItemSuffix::IncreasedLightningDamage, 500),
	];

	let green_suffix_weights = [
		(ItemSuffix::Armour, 50),
		(ItemSuffix::PhysicalResistance, 50),
		(ItemSuffix::ColdResistance, 500),
		(ItemSuffix::FireResistance, 500),
		(ItemSuffix::LightningResistance, 1000),
		(ItemSuffix::CriticalMultiplier, 500),
		(ItemSuffix::IncreasedDamage, 50),
		(ItemSuffix::IncreasedColdDamage, 500),
		(ItemSuffix::IncreasedFireDamage, 500),
		(ItemSuffix::IncreasedLightningDamage, 1000),
	];

	let blue_suffix_weights = [
		(ItemSuffix::Armour, 1000),
		(ItemSuffix::PhysicalResistance, 100),
		(ItemSuffix::ColdResistance, 1000),
		(ItemSuffix::FireResistance, 500),
		(ItemSuffix::LightningResistance, 500),
		(ItemSuffix::CriticalMultiplier, 100),
		(ItemSuffix::IncreasedDamage, 50),
		(ItemSuffix::IncreasedColdDamage, 1000),
		(ItemSuffix::IncreasedFireDamage, 500),
		(ItemSuffix::IncreasedLightningDamage, 500),
	];

	let prefix_weights = [
		red_prefix_weights,
		green_prefix_weights,
		blue_prefix_weights,
	][kind as usize];
	let suffix_weights = [
		red_suffix_weights,
		green_suffix_weights,
		blue_suffix_weights,
	][kind as usize];

	let num_affixes = [1, 3][rarity as usize];
	let num_prefixes = rng.gen_range(0..=num_affixes);
	let num_suffixes = if num_prefixes == 0
	{
		rng.gen_range(1..=num_affixes)
	}
	else
	{
		rng.gen_range(0..=num_affixes)
	};

	let mut prefixes: Vec<(_, i32, f32)> = vec![];
	for _ in 0..num_prefixes
	{
		loop
		{
			let prefix = prefix_weights.choose_weighted(rng, |&(_, w)| w).unwrap().0;
			if prefixes.iter().find(|p| p.0 == prefix).is_none()
			{
				prefixes.push((prefix, 0, rng.gen_range(0.0..1.0f32)));
				break;
			}
		}
	}

	let mut suffixes: Vec<(_, i32, f32)> = vec![];
	for _ in 0..num_suffixes
	{
		loop
		{
			let suffix = suffix_weights.choose_weighted(rng, |&(_, w)| w).unwrap().0;
			if suffixes.iter().find(|p| p.0 == suffix).is_none()
			{
				suffixes.push((suffix, 0, rng.gen_range(0.0..1.0f32)));
				break;
			}
		}
	}

	let name = match rarity
	{
		Rarity::Magic =>
		{
			make_magic_name(kind, prefixes.first().copied(), suffixes.first().copied())
		}
		Rarity::Rare => make_rare_name(rng),
	};

	let appearance = Appearance::new("data/ring_red.cfg");
	Item {
		name: name,
		appearance: appearance,
		prefixes: prefixes,
		suffixes: suffixes,
	}
}

fn make_magic_name(
	kind: ItemKind, prefix: Option<(ItemPrefix, i32, f32)>, suffix: Option<(ItemSuffix, i32, f32)>,
) -> Vec<String>
{
	let prefix = prefix.map(|(a, t, _)| a.to_str(t)).unwrap_or("");
	let suffix = suffix.map(|(a, t, _)| a.to_str(t)).unwrap_or("");
	vec![
		prefix.to_string(),
		kind.to_str().to_string(),
		suffix.to_string(),
	]
}

fn make_rare_name(rng: &mut impl Rng) -> Vec<String>
{
	vec!["Rare".to_string()]
}
