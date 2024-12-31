use crate::error::Result;
use crate::utils::DT;
use crate::{game_state, sprite, utils};
use allegro::*;
use na::{Point2, Point3, Vector2, Vector3};
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
	pub ground_pos: Vector3<f32>,
}

impl Velocity
{
	pub fn new(pos: Vector3<f32>) -> Self
	{
		Self {
			pos: pos,
			ground_pos: Vector3::zeros(),
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Acceleration
{
	pub pos: Vector3<f32>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[repr(i32)]
pub enum Material
{
	Default = 0,
	Frozen = 1,
	Lit = 2,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Appearance
{
	pub sprite: String,
	pub palette: Option<String>,
	pub animation_state: sprite::AnimationState,
	pub material: Material,
	pub speed: f32,
	pub bias: i32,
}

impl Appearance
{
	pub fn new_with_bias(sprite: impl Into<String>, bias: i32) -> Self
	{
		Self {
			sprite: sprite.into(),
			palette: None,
			animation_state: sprite::AnimationState::new("Default"),
			speed: 1.,
			material: Material::Default,
			bias,
		}
	}

	pub fn new(sprite: impl Into<String>) -> Self
	{
		Self::new_with_bias(sprite, 0)
	}
}

#[derive(Debug, Clone)]
pub struct StatusAppearance
{
	pub shocked: Option<Appearance>,
	pub ignited: Option<Appearance>,
	pub persistent: Vec<Appearance>,
}

impl StatusAppearance
{
	pub fn new() -> Self
	{
		Self {
			shocked: None,
			ignited: None,
			persistent: vec![],
		}
	}

	pub fn new_with_effects(persistent: Vec<Appearance>) -> Self
	{
		Self {
			shocked: None,
			ignited: None,
			persistent,
		}
	}

	pub fn ignite(&mut self, apply: bool)
	{
		if apply
		{
			if self.ignited.is_none()
			{
				self.ignited = Some(Appearance::new_with_bias("data/ignited.cfg", 1));
			}
		}
		else
		{
			self.ignited = None;
		}
	}

	pub fn shock(&mut self, apply: bool)
	{
		if apply
		{
			if self.shocked.is_none()
			{
				self.shocked = Some(Appearance::new_with_bias("data/shocked.cfg", 1));
			}
		}
		else
		{
			self.shocked = None;
		}
	}
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CollisionKind
{
	BigEnemy,
	BigPlayer,
	SmallEnemy,
	SmallPlayer,
	World,
	Platform,
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
			(CollisionKind::BigEnemy, CollisionKind::Platform) => true,

			(CollisionKind::BigPlayer, CollisionKind::BigEnemy) => true,
			(CollisionKind::BigPlayer, CollisionKind::BigPlayer) => true,
			(CollisionKind::BigPlayer, CollisionKind::SmallEnemy) => true,
			(CollisionKind::BigPlayer, CollisionKind::SmallPlayer) => false,
			(CollisionKind::BigPlayer, CollisionKind::World) => true,
			(CollisionKind::BigPlayer, CollisionKind::Platform) => true,

			(CollisionKind::SmallEnemy, CollisionKind::BigEnemy) => false,
			(CollisionKind::SmallEnemy, CollisionKind::BigPlayer) => true,
			(CollisionKind::SmallEnemy, CollisionKind::SmallEnemy) => false,
			(CollisionKind::SmallEnemy, CollisionKind::SmallPlayer) => false,
			(CollisionKind::SmallEnemy, CollisionKind::World) => true,
			(CollisionKind::SmallEnemy, CollisionKind::Platform) => true,

			(CollisionKind::SmallPlayer, CollisionKind::BigEnemy) => true,
			(CollisionKind::SmallPlayer, CollisionKind::BigPlayer) => false,
			(CollisionKind::SmallPlayer, CollisionKind::SmallEnemy) => false,
			(CollisionKind::SmallPlayer, CollisionKind::SmallPlayer) => false,
			(CollisionKind::SmallPlayer, CollisionKind::World) => true,
			(CollisionKind::SmallPlayer, CollisionKind::Platform) => true,

			(CollisionKind::World, CollisionKind::BigEnemy) => true,
			(CollisionKind::World, CollisionKind::BigPlayer) => true,
			(CollisionKind::World, CollisionKind::SmallEnemy) => true,
			(CollisionKind::World, CollisionKind::SmallPlayer) => true,
			(CollisionKind::World, CollisionKind::World) => false,
			(CollisionKind::World, CollisionKind::Platform) => false,

			(CollisionKind::Platform, CollisionKind::BigEnemy) => true,
			(CollisionKind::Platform, CollisionKind::BigPlayer) => true,
			(CollisionKind::Platform, CollisionKind::SmallEnemy) => true,
			(CollisionKind::Platform, CollisionKind::SmallPlayer) => true,
			(CollisionKind::Platform, CollisionKind::World) => false,
			(CollisionKind::Platform, CollisionKind::Platform) => false,
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
	pub attack_range: f32,
	pub leash: Point3<f32>,
}

impl AI
{
	pub fn new_ranged(leash: Point3<f32>) -> Self
	{
		Self {
			state: AIState::Idle,
			next_state_time: 0.,
			attack_range: 96.,
			target: None,
			leash: leash,
		}
	}

	pub fn new_melee(leash: Point3<f32>) -> Self
	{
		Self {
			state: AIState::Idle,
			next_state_time: 0.,
			attack_range: 24.,
			target: None,
			leash: leash,
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
	pub team: Team,

	pub max_life: f32,
	pub life_regen: f32,
	pub max_mana: f32,
	pub mana_regen: f32,

	pub armor: f32,

	pub area_of_effect: f32,
	pub cast_speed: f32,
	pub skill_duration: f32,
	pub critical_chance: f32,
	pub critical_multiplier: f32,

	pub physical_damage: f32,
	pub cold_damage: f32,
	pub fire_damage: f32,
	pub lightning_damage: f32,

	pub physical_resistance: f32,
	pub cold_resistance: f32,
	pub fire_resistance: f32,
	pub lightning_resistance: f32,

	pub life_leech: f32,
	pub mana_leech: f32,
	pub chance_to_ignite: f32,
	pub chance_to_freeze: f32,
	pub chance_to_shock: f32,

	pub freeze_propagate_value: f32,
	pub ignite_propagate_value: EffectAndDuration,
	pub shock_propagate_value: EffectAndDuration,

	pub increased_physical_damage: f32,
	pub multishot: bool,
	pub explode_on_death: bool,
	pub freeze_propagate: bool,
	pub ignite_propagate: bool,
	pub shock_propagate: bool,

	pub is_invincible: bool,
}

impl Default for StatValues
{
	fn default() -> Self
	{
		Self {
			speed: 0.,
			acceleration: 0.,
			jump_strength: 0.,
			team: Team::Enemy,

			max_life: 0.,
			life_regen: 0.,
			max_mana: 0.,
			mana_regen: 0.,

			armor: 0.,

			area_of_effect: 0.,
			cast_speed: 0.,
			skill_duration: 0.,
			critical_chance: 0.,
			critical_multiplier: 0.,

			physical_damage: 0.,
			cold_damage: 0.,
			fire_damage: 0.,
			lightning_damage: 0.,

			physical_resistance: 0.,
			cold_resistance: 0.,
			fire_resistance: 0.,
			lightning_resistance: 0.,

			life_leech: 0.,
			mana_leech: 0.,
			chance_to_ignite: 0.,
			chance_to_freeze: 0.,
			chance_to_shock: 0.,

			freeze_propagate_value: 0.,
			ignite_propagate_value: EffectAndDuration::new(),
			shock_propagate_value: EffectAndDuration::new(),

			increased_physical_damage: 0.,
			multishot: false,
			explode_on_death: false,
			freeze_propagate: false,
			ignite_propagate: false,
			shock_propagate: false,

			is_invincible: false,
		}
	}
}

impl StatValues
{
	pub fn new_player() -> Self
	{
		Self {
			speed: 136.,
			acceleration: 512.,
			jump_strength: 128.,
			team: Team::Player,

			max_life: 100.,
			life_regen: 5.,
			max_mana: 100.,
			mana_regen: 5.,

			area_of_effect: 1.,
			cast_speed: 1.,
			skill_duration: 1.,

			critical_chance: 0.05,
			critical_multiplier: 2.,
			physical_damage: 10.,

			//freeze_propagate: true,
			//ignite_propagate: true,
			//shock_propagate: true,
			//explode_on_death: true,
			//fire_damage: 10.,
			//chance_to_ignite: 1.,
			//lightning_damage: 10.,
			//chance_to_shock: 1.,
			//cold_damage: 10.,
			//chance_to_freeze: 1.,
			..Self::default()
		}
	}

	pub fn new_enemy(level: i32, rarity: Rarity, ranged: bool) -> Self
	{
		let mut f = match rarity
		{
			Rarity::Normal => 1.,
			Rarity::Magic => 1.5,
			Rarity::Rare => 3.,
			Rarity::Unique => 10.,
		};

		if !ranged
		{
			f *= 1.5;
		}

		Self {
			speed: if ranged { 64. } else { 96. },
			acceleration: if ranged { 512. } else { 1400. },
			skill_duration: 1.,
			max_life: (42. + 10. * level as f32) * 1.1_f32.powf(level as f32) * f,
			mana_regen: 100.,
			max_mana: 100.,
			cast_speed: 1.,
			critical_chance: 0.05,
			critical_multiplier: 1.5,

			physical_damage: (-7. + 0. * level as f32 + 10. * 1.1_f32.powf(level as f32)) * f,

			area_of_effect: 1.,
			..Self::default()
		}
	}

	pub fn new_fireball() -> Self
	{
		Self {
			speed: 256.,
			acceleration: 1024.,
			max_life: 1.,
			is_invincible: true,
			..Self::default()
		}
	}

	pub fn new_item() -> Self
	{
		Self {
			speed: 256.,
			team: Team::Neutral,
			jump_strength: 128.,
			max_life: 1.,
			is_invincible: true,
			..Self::default()
		}
	}

	pub fn new_corpse() -> Self
	{
		Self {
			speed: 256.,
			team: Team::Neutral,
			max_life: 1.,
			is_invincible: true,
			..Self::default()
		}
	}

	pub fn new_platform() -> Self
	{
		Self {
			speed: 256.,
			team: Team::Neutral,
			max_life: 1.,
			is_invincible: true,
			..Self::default()
		}
	}
}

#[derive(Debug, Clone)]
pub struct RateInstance
{
	pub rate: f32,
	pub time_to_remove: f64,
}

#[derive(Debug, Copy, Clone)]
pub struct EffectAndDuration
{
	pub effect: f32,
	pub duration: f32,
}

impl EffectAndDuration
{
	pub fn new() -> Self
	{
		Self {
			effect: 0.,
			duration: 0.,
		}
	}

	pub fn active(&self) -> bool
	{
		self.effect > 0. && self.duration > 0.
	}
}

#[derive(Debug, Clone)]
pub struct Stats
{
	pub base_values: StatValues,
	pub values: StatValues,

	pub attacking: bool,
	pub life: f32,
	pub mana: f32,
	pub old_max_life: f32,
	pub old_max_mana: f32,
	pub dead: bool,
	pub exploded: bool,

	pub life_leech_instances: Vec<RateInstance>,
	pub mana_leech_instances: Vec<RateInstance>,

	pub ignite_instances: Vec<RateInstance>,
	pub shock_instances: Vec<RateInstance>,
	pub freeze_time: f64,
}

impl Stats
{
	pub fn new(base_values: StatValues) -> Self
	{
		Self {
			base_values: base_values,
			values: base_values,
			attacking: false,
			life: base_values.max_life,
			mana: base_values.max_mana,
			old_max_life: base_values.max_life,
			old_max_mana: base_values.max_mana,
			dead: false,
			life_leech_instances: vec![],
			mana_leech_instances: vec![],
			ignite_instances: vec![],
			shock_instances: vec![],
			freeze_time: 0.,
			exploded: false,
		}
	}

	pub fn reset(&mut self, time: f64, penalty_level: i32, inventory: Option<&Inventory>)
	{
		let penalty = (penalty_level / 5) as f32;

		self.values = self.base_values;
		if let Some(inventory) = inventory
		{
			let mut adds = StatValues::default();
			let mut increases = StatValues::default();
			for slot in &inventory.slots[..6]
			{
				if let Some(item) = slot
				{
					for (prefix, tier, frac) in &item.prefixes
					{
						prefix.apply(*tier, *frac, &mut adds, &mut increases);
					}
					for (suffix, tier, frac) in &item.suffixes
					{
						suffix.apply(*tier, *frac, &mut adds, &mut increases);
					}
				}
			}
			self.values.speed = (self.base_values.speed + adds.speed) * (1. + increases.speed);
			self.values.acceleration =
				(self.base_values.acceleration + adds.acceleration) * (1. + increases.acceleration);
			self.values.jump_strength = (self.base_values.jump_strength + adds.jump_strength)
				* (1. + increases.jump_strength);

			self.values.max_life =
				(self.base_values.max_life + adds.max_life) * (1. + increases.max_life);
			self.values.life_regen =
				(self.base_values.life_regen + adds.life_regen) * (1. + increases.life_regen);
			self.values.max_mana =
				(self.base_values.max_mana + adds.max_mana) * (1. + increases.max_mana);
			self.values.mana_regen =
				(self.base_values.mana_regen + adds.mana_regen) * (1. + increases.mana_regen);

			self.values.armor = (self.base_values.armor + adds.armor) * (1. + increases.armor);

			self.values.area_of_effect = (self.base_values.area_of_effect + adds.area_of_effect)
				* (1. + increases.area_of_effect);
			self.values.cast_speed =
				(self.base_values.cast_speed + adds.cast_speed) * (1. + increases.cast_speed);
			self.values.skill_duration = (self.base_values.skill_duration + adds.skill_duration)
				* (1. + increases.skill_duration);
			self.values.critical_chance = (self.base_values.critical_chance + adds.critical_chance)
				* (1. + increases.critical_chance);
			self.values.critical_multiplier = (self.base_values.critical_multiplier
				+ adds.critical_multiplier)
				* (1. + increases.critical_multiplier);

			self.values.physical_damage = (self.base_values.physical_damage + adds.physical_damage)
				* (1. + increases.physical_damage);
			self.values.increased_physical_damage = increases.physical_damage;
			self.values.cold_damage =
				(self.base_values.cold_damage + adds.cold_damage) * (1. + increases.cold_damage);
			self.values.fire_damage =
				(self.base_values.fire_damage + adds.fire_damage) * (1. + increases.fire_damage);
			self.values.lightning_damage = (self.base_values.lightning_damage
				+ adds.lightning_damage)
				* (1. + increases.lightning_damage);

			self.values.physical_resistance = (self.base_values.physical_resistance
				+ adds.physical_resistance)
				* (1. + increases.physical_resistance);
			self.values.cold_resistance = (self.base_values.cold_resistance + adds.cold_resistance)
				* (1. + increases.cold_resistance);
			self.values.fire_resistance = (self.base_values.fire_resistance + adds.fire_resistance)
				* (1. + increases.fire_resistance);
			self.values.lightning_resistance = (self.base_values.lightning_resistance
				+ adds.lightning_resistance)
				* (1. + increases.lightning_resistance);

			self.values.life_leech =
				(self.base_values.life_leech + adds.life_leech) * (1. + increases.life_leech);
			self.values.mana_leech =
				(self.base_values.mana_leech + adds.mana_leech) * (1. + increases.mana_leech);
			self.values.chance_to_ignite = (self.base_values.chance_to_ignite
				+ adds.chance_to_ignite)
				* (1. + increases.chance_to_ignite);
			self.values.chance_to_freeze = (self.base_values.chance_to_freeze
				+ adds.chance_to_freeze)
				* (1. + increases.chance_to_freeze);
			self.values.chance_to_shock = (self.base_values.chance_to_shock + adds.chance_to_shock)
				* (1. + increases.chance_to_shock);

			self.values.multishot |= adds.multishot;
			self.values.explode_on_death |= adds.explode_on_death;
			self.values.shock_propagate |= adds.shock_propagate;
			self.values.ignite_propagate |= adds.ignite_propagate;
			self.values.freeze_propagate |= adds.freeze_propagate;

			self.values.critical_chance = utils::min(1., self.values.critical_chance);

			self.values.physical_resistance = utils::min(0.9, self.values.physical_resistance);
			self.values.cold_resistance =
				utils::clamp(self.values.cold_resistance - penalty * 0.3, -1., 0.75);
			self.values.fire_resistance =
				utils::clamp(self.values.fire_resistance - penalty * 0.3, -1., 0.75);
			self.values.lightning_resistance =
				utils::clamp(self.values.lightning_resistance - penalty * 0.3, -1., 0.75);

			self.values.chance_to_shock = utils::min(1., self.values.chance_to_shock);
			self.values.chance_to_ignite = utils::min(1., self.values.chance_to_ignite);
			self.values.chance_to_freeze = utils::min(1., self.values.chance_to_freeze);

			self.life *= self.values.max_life / self.old_max_life;
			self.life = utils::min(self.values.max_life, self.life);
			self.old_max_life = self.values.max_life;

			self.mana *= self.values.max_mana / self.old_max_mana;
			self.mana = utils::min(self.values.max_mana, self.mana);
			self.old_max_mana = self.values.max_mana;
		}

		if self.attacking
		{
			self.values.acceleration = 0.;
			self.values.jump_strength = 0.;
		}
		if self.dead
		{
			self.values.team = Team::Neutral;
			self.life = 1.;
		}
		if self.freeze_time > time
		{
			self.values.cast_speed = 0.;
			self.values.acceleration = 0.;
			self.values.jump_strength = 0.;
		}
	}

	pub fn apply_damage(
		&mut self, values: &StatValues, state: &mut game_state::GameState, rng: &mut impl Rng,
	) -> (f32, f32, bool, f32, EffectAndDuration, EffectAndDuration)
	{
		if self.dead || self.values.is_invincible
		{
			return (
				0.,
				0.,
				false,
				0.,
				EffectAndDuration::new(),
				EffectAndDuration::new(),
			);
		}
		let (crit, damage_mult) = if rng.gen_bool(values.critical_chance as f64)
		{
			(true, values.critical_multiplier)
		}
		else
		{
			(false, 1.)
		};

		let shock_damage = (1. - self.values.lightning_resistance)
			* self
				.shock_instances
				.iter()
				.map(|li| li.rate)
				.reduce(utils::max)
				.unwrap_or(0.);
		let shock_effect = shock_damage / self.values.max_life;

		let damage_mult = if shock_effect > 0.
		{
			damage_mult * (1. + shock_effect)
		}
		else
		{
			damage_mult
		};

		let old_frozen = state.time() < self.freeze_time;
		let mut freeze_duration = values.freeze_propagate_value;
		if values.cold_damage > 0. && (crit || rng.gen_bool(values.chance_to_freeze as f64))
		{
			freeze_duration = 10.
				* damage_mult
				* values.skill_duration
				* values.cold_damage
				* (1. - self.values.cold_resistance)
				/ self.values.max_life;
		}
		if freeze_duration > 0.1
		{
			self.freeze_time = state.time() + freeze_duration as f64;
		}

		let old_ignited = !self.ignite_instances.is_empty();
		let mut ignite = values.ignite_propagate_value;
		if values.fire_damage > 0. && (crit || rng.gen_bool(values.chance_to_ignite as f64))
		{
			ignite = EffectAndDuration {
				effect: damage_mult * values.fire_damage * DT,
				duration: values.skill_duration * 2.,
			};
		}
		if ignite.active()
		{
			self.ignite_instances.push(RateInstance {
				rate: ignite.effect,
				time_to_remove: state.time() + ignite.duration as f64,
			});
		}

		let old_shocked = !self.shock_instances.is_empty();
		let mut shock = values.shock_propagate_value;
		if values.lightning_damage > 0. && (crit || rng.gen_bool(values.chance_to_shock as f64))
		{
			shock = EffectAndDuration {
				effect: damage_mult * values.lightning_damage,
				duration: values.skill_duration * 2.,
			};
		}
		if shock.active()
		{
			self.shock_instances.push(RateInstance {
				rate: shock.effect,
				time_to_remove: state.time() + shock.duration as f64,
			})
		}

		let extra_phys_resistance = if self.values.armor > 0.
		{
			self.values.armor / (values.physical_damage + self.values.armor)
		}
		else
		{
			0.
		};
		let phys_resistance =
			utils::min(0.9, self.values.physical_resistance + extra_phys_resistance);
		let damage = values.physical_damage * (1. - phys_resistance)
			+ values.cold_damage * (1. - self.values.cold_resistance)
			+ values.fire_damage * (1. - self.values.fire_resistance)
			+ values.lightning_damage * (1. - self.values.lightning_resistance);
		let final_damage = damage_mult * damage;
		let life_leech = final_damage * values.life_leech;
		let mana_leech = final_damage * values.mana_leech;
		self.life = utils::max(0., self.life - final_damage);
		let explode_on_death = if self.life == 0. && values.explode_on_death
		{
			self.exploded = true;
			true
		}
		else
		{
			false
		};

		let freeze_propagation = if values.freeze_propagate && !old_frozen
		{
			freeze_duration
		}
		else
		{
			0.
		};
		let ignite_propagation = if values.ignite_propagate && !old_ignited
		{
			ignite
		}
		else
		{
			EffectAndDuration::new()
		};
		let shock_propagation = if values.shock_propagate && !old_shocked
		{
			shock
		}
		else
		{
			EffectAndDuration::new()
		};
		(
			life_leech,
			mana_leech,
			explode_on_death,
			freeze_propagation,
			ignite_propagation,
			shock_propagation,
		)
	}

	pub fn logic(&mut self, state: &mut game_state::GameState)
	{
		self.life_leech_instances
			.retain_mut(|li| li.time_to_remove > state.time());
		self.mana_leech_instances
			.retain_mut(|li| li.time_to_remove > state.time());
		self.ignite_instances
			.retain_mut(|li| li.time_to_remove > state.time());
		self.shock_instances
			.retain_mut(|li| li.time_to_remove > state.time());

		let life_leech = self
			.life_leech_instances
			.iter()
			.map(|li| li.rate)
			.reduce(utils::max)
			.unwrap_or(0.);
		let mana_leech = self
			.mana_leech_instances
			.iter()
			.map(|li| li.rate)
			.reduce(utils::max)
			.unwrap_or(0.);
		let ignite_damage = self
			.ignite_instances
			.iter()
			.map(|li| li.rate)
			.reduce(utils::max)
			.unwrap_or(0.);

		self.life += life_leech - ignite_damage * (1. - self.values.fire_resistance);
		self.mana += mana_leech;

		if self.life >= self.values.max_life
		{
			self.life_leech_instances.clear();
		}
		if self.mana >= self.values.max_mana
		{
			self.mana_leech_instances.clear();
		}
		self.life += self.values.life_regen * DT;
		self.mana += self.values.mana_regen * DT;
		self.life = utils::clamp(self.life, 0., self.values.max_life);
		self.mana = utils::clamp(self.mana, 0., self.values.max_mana);
	}
}

#[derive(Debug, Copy, Clone)]
pub enum AttackKind
{
	BladeBlade,
	Slam,
	Fireball(Rarity),
	Explode,
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

#[derive(Debug, Clone)]
pub enum Effect
{
	Die,
	SpawnExplosion(String, Color, &'static str),
	DoDamage(StatValues, Team),
	SpawnCorpse(&'static str),
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemPrefix
{
	Life,
	LifeRegen,
	AddedPhysicalDamage,
	AddedColdDamage,
	AddedFireDamage,
	AddedLightningDamage,
	CriticalChance,
	ChanceToFreeze,
	ChanceToIgnite,
	ChanceToShock,
	Mana,
	ManaRegen,
	AreaOfEffect,
	CastSpeed,
	MoveSpeed,
	MultiShot,
	ExplodeOnDeath,
	FreezePropagate,
	IgnitePropagate,
	ShockPropagate,
}

impl ItemPrefix
{
	pub fn to_str(&self) -> &'static str
	{
		match self
		{
			ItemPrefix::Life => "Jolly",
			ItemPrefix::LifeRegen => "Hale",
			ItemPrefix::AddedPhysicalDamage => "Bladed",
			ItemPrefix::AddedColdDamage => "Icy",
			ItemPrefix::AddedFireDamage => "Blazing",
			ItemPrefix::AddedLightningDamage => "Sparking",
			ItemPrefix::CriticalChance => "Elven",
			ItemPrefix::ChanceToFreeze => "Chilling",
			ItemPrefix::ChanceToIgnite => "Flaming",
			ItemPrefix::ChanceToShock => "Zapping",
			ItemPrefix::Mana => "Clever",
			ItemPrefix::ManaRegen => "Meditating",
			ItemPrefix::AreaOfEffect => "Engorged",
			ItemPrefix::CastSpeed => "Animated",
			ItemPrefix::MoveSpeed => "Fast",
			ItemPrefix::MultiShot => "MultiShot",
			ItemPrefix::ExplodeOnDeath => "ExplodeOnDeath",
			ItemPrefix::FreezePropagate => "FreezePropagate",
			ItemPrefix::IgnitePropagate => "IgnitePropagate",
			ItemPrefix::ShockPropagate => "ShockPropagate",
		}
	}

	pub fn get_value(&self, tier: i32, frac: f32) -> f32
	{
		let tier = tier as f32;
		let (delta, mult) = match self
		{
			ItemPrefix::Life => (20., 1.),
			ItemPrefix::LifeRegen => (1., 1.),
			ItemPrefix::AddedPhysicalDamage => (2., 1.),
			ItemPrefix::AddedColdDamage => (2., 1.),
			ItemPrefix::AddedFireDamage => (2., 1.),
			ItemPrefix::AddedLightningDamage => (2., 1.),
			ItemPrefix::CriticalChance => (0.1, 0.01),
			ItemPrefix::ChanceToFreeze => (0.02, 0.01),
			ItemPrefix::ChanceToIgnite => (0.02, 0.01),
			ItemPrefix::ChanceToShock => (0.02, 0.01),
			ItemPrefix::Mana => (20., 1.),
			ItemPrefix::ManaRegen => (2., 1.),
			ItemPrefix::AreaOfEffect => (0.05, 0.01),
			ItemPrefix::CastSpeed => (0.05, 0.01),
			ItemPrefix::MoveSpeed => (0.01, 0.01),
			ItemPrefix::MultiShot => (0.1, 0.01),
			ItemPrefix::ExplodeOnDeath => (0.1, 0.01),
			ItemPrefix::FreezePropagate => (0.1, 0.01),
			ItemPrefix::IgnitePropagate => (0.1, 0.01),
			ItemPrefix::ShockPropagate => (0.1, 0.01),
		};
		let start = delta * tier;
		let end = delta * (tier + 1.);
		let raw = start + frac * (end - start);
		(raw / mult).ceil() * mult
	}

	pub fn get_mod_string(&self, tier: i32, frac: f32) -> String
	{
		let value = self.get_value(tier, frac);
		let add_percent = match self
		{
			ItemPrefix::Life
			| ItemPrefix::LifeRegen
			| ItemPrefix::AddedPhysicalDamage
			| ItemPrefix::AddedColdDamage
			| ItemPrefix::AddedFireDamage
			| ItemPrefix::AddedLightningDamage
			| ItemPrefix::Mana => false,
			ItemPrefix::ManaRegen => false,
			_ => true,
		};
		let percent = if add_percent { "%" } else { "" };
		let value = if add_percent { value * 100. } else { value };
		let sign = if value > 0. { "+" } else { "-" };
		let suffix = match self
		{
			ItemPrefix::Life => "Max Life",
			ItemPrefix::LifeRegen => "Life Regen",
			ItemPrefix::AddedPhysicalDamage => "Physical Damage",
			ItemPrefix::AddedColdDamage => "Cold Damage",
			ItemPrefix::AddedFireDamage => "Fire Damage",
			ItemPrefix::AddedLightningDamage => "Lightning Damage",
			ItemPrefix::CriticalChance => "Critical Chance",
			ItemPrefix::ChanceToFreeze => "Freeze Chance",
			ItemPrefix::ChanceToIgnite => "Ignite Chance",
			ItemPrefix::ChanceToShock => "Shock Chance",
			ItemPrefix::Mana => "Max Mana",
			ItemPrefix::ManaRegen => "Mana Regen",
			ItemPrefix::AreaOfEffect => "Area of Effect",
			ItemPrefix::CastSpeed => "Cast Speed",
			ItemPrefix::MoveSpeed => "Move Speed",
			ItemPrefix::MultiShot => "Multiple Shots",
			ItemPrefix::ExplodeOnDeath => "Enemies Explode",
			ItemPrefix::FreezePropagate => "Freezes Spread",
			ItemPrefix::IgnitePropagate => "Ignites Spread",
			ItemPrefix::ShockPropagate => "Shocks Spread",
		};
		let unique = match self
		{
			ItemPrefix::ExplodeOnDeath
			| ItemPrefix::FreezePropagate
			| ItemPrefix::ShockPropagate
			| ItemPrefix::IgnitePropagate => true,
			_ => false,
		};
		if unique
		{
			suffix.to_string()
		}
		else
		{
			format!(
				"{sign}{value}{percent} {suffix}",
				value = utils::nice_float(value, 1)
			)
		}
	}
	pub fn apply(&self, tier: i32, frac: f32, adds: &mut StatValues, increases: &mut StatValues)
	{
		let value = self.get_value(tier, frac);
		match self
		{
			ItemPrefix::Life =>
			{
				adds.max_life += value;
			}
			ItemPrefix::LifeRegen =>
			{
				adds.life_regen += value;
			}
			ItemPrefix::AddedPhysicalDamage =>
			{
				adds.physical_damage += value;
			}
			ItemPrefix::AddedColdDamage =>
			{
				adds.cold_damage += value;
			}
			ItemPrefix::AddedFireDamage =>
			{
				adds.fire_damage += value;
			}
			ItemPrefix::AddedLightningDamage =>
			{
				adds.lightning_damage += value;
			}
			ItemPrefix::CriticalChance =>
			{
				increases.critical_chance += value;
			}
			ItemPrefix::ChanceToFreeze =>
			{
				adds.chance_to_freeze += value;
			}
			ItemPrefix::ChanceToIgnite =>
			{
				adds.chance_to_ignite += value;
			}
			ItemPrefix::ChanceToShock =>
			{
				adds.chance_to_shock += value;
			}
			ItemPrefix::Mana =>
			{
				adds.max_mana += value;
			}
			ItemPrefix::ManaRegen =>
			{
				adds.mana_regen += value;
			}
			ItemPrefix::AreaOfEffect =>
			{
				increases.area_of_effect += value;
			}
			ItemPrefix::CastSpeed =>
			{
				increases.cast_speed += value;
			}
			ItemPrefix::MoveSpeed =>
			{
				increases.speed += value;
			}
			ItemPrefix::MultiShot =>
			{
				adds.multishot = true;
			}
			ItemPrefix::ExplodeOnDeath =>
			{
				adds.explode_on_death = true;
			}
			ItemPrefix::FreezePropagate =>
			{
				adds.freeze_propagate = true;
			}
			ItemPrefix::IgnitePropagate =>
			{
				adds.ignite_propagate = true;
			}
			ItemPrefix::ShockPropagate =>
			{
				adds.shock_propagate = true;
			}
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i32)]
pub enum ItemSuffix
{
	Armour = 0,
	PhysicalResistance = 1,
	ColdResistance = 2,
	FireResistance = 3,
	LightningResistance = 4,
	CriticalMultiplier = 5,
	IncreasedPhysicalDamage = 6,
	IncreasedColdDamage = 7,
	IncreasedFireDamage = 8,
	IncreasedLightningDamage = 9,
	LifeLeech = 10,
	ManaLeech = 11,
	Duration = 12,
}

impl ItemSuffix
{
	pub fn to_str(&self) -> &'static str
	{
		match self
		{
			ItemSuffix::Armour => "of Iron",
			ItemSuffix::PhysicalResistance => "of Fortitude",
			ItemSuffix::ColdResistance => "of the Penguin",
			ItemSuffix::FireResistance => "of the Salamander",
			ItemSuffix::LightningResistance => "of the Eel",
			ItemSuffix::CriticalMultiplier => "of Misfortune",
			ItemSuffix::IncreasedPhysicalDamage => "of Blades",
			ItemSuffix::IncreasedColdDamage => "of the Iceberg",
			ItemSuffix::IncreasedFireDamage => "of the Hearth",
			ItemSuffix::IncreasedLightningDamage => "of the Turbine",
			ItemSuffix::LifeLeech => "of the Vampire",
			ItemSuffix::ManaLeech => "of the Wight",
			ItemSuffix::Duration => "of Time",
		}
	}

	pub fn get_value(&self, tier: i32, frac: f32) -> f32
	{
		let tier = tier as f32;
		let (delta, mult) = match self
		{
			ItemSuffix::Armour => (5., 1.),
			ItemSuffix::PhysicalResistance => (0.01, 0.01),
			ItemSuffix::ColdResistance => (0.1, 0.01),
			ItemSuffix::FireResistance => (0.1, 0.01),
			ItemSuffix::LightningResistance => (0.1, 0.01),
			ItemSuffix::CriticalMultiplier => (0.05, 0.01),
			ItemSuffix::IncreasedPhysicalDamage => (0.05, 0.01),
			ItemSuffix::IncreasedColdDamage => (0.10, 0.01),
			ItemSuffix::IncreasedFireDamage => (0.10, 0.01),
			ItemSuffix::IncreasedLightningDamage => (0.10, 0.01),
			ItemSuffix::LifeLeech => (0.01, 0.01),
			ItemSuffix::ManaLeech => (0.01, 0.01),
			ItemSuffix::Duration => (0.02, 0.01),
		};
		let start = delta * tier;
		let end = delta * (tier + 1.);
		let raw = start + frac * (end - start);
		(raw / mult).ceil() * mult
	}

	pub fn get_mod_string(&self, tier: i32, frac: f32) -> String
	{
		let value = self.get_value(tier, frac);
		let add_percent = match self
		{
			ItemSuffix::Armour => false,
			_ => true,
		};
		let percent = if add_percent { "%" } else { "" };
		let value = if add_percent { value * 100. } else { value };
		let sign = if value > 0. { "+" } else { "-" };
		let suffix = match self
		{
			ItemSuffix::Armour => "Armour",
			ItemSuffix::PhysicalResistance => "Physical Resist",
			ItemSuffix::ColdResistance => "Cold Resist",
			ItemSuffix::FireResistance => "Fire Resist",
			ItemSuffix::LightningResistance => "Lightning Resist",
			ItemSuffix::CriticalMultiplier => "Critical Multi",
			ItemSuffix::IncreasedPhysicalDamage => "Physical Damage",
			ItemSuffix::IncreasedColdDamage => "Cold Damage",
			ItemSuffix::IncreasedFireDamage => "Fire Damage",
			ItemSuffix::IncreasedLightningDamage => "Lightning Damage",
			ItemSuffix::LifeLeech => "Life Leech",
			ItemSuffix::ManaLeech => "Mana Leech",
			ItemSuffix::Duration => "Skill Duration",
		};
		format!(
			"{sign}{value}{percent} {suffix}",
			value = utils::nice_float(value, 1)
		)
	}

	pub fn apply(&self, tier: i32, frac: f32, adds: &mut StatValues, increases: &mut StatValues)
	{
		let value = self.get_value(tier, frac);
		match self
		{
			ItemSuffix::Armour =>
			{
				adds.armor += value;
			}
			ItemSuffix::PhysicalResistance =>
			{
				adds.physical_resistance += value;
			}
			ItemSuffix::ColdResistance =>
			{
				adds.cold_resistance += value;
			}
			ItemSuffix::FireResistance =>
			{
				adds.fire_resistance += value;
			}
			ItemSuffix::LightningResistance =>
			{
				adds.lightning_resistance += value;
			}
			ItemSuffix::CriticalMultiplier =>
			{
				adds.critical_multiplier += value;
			}
			ItemSuffix::IncreasedPhysicalDamage =>
			{
				increases.physical_damage += value;
			}
			ItemSuffix::IncreasedColdDamage =>
			{
				increases.cold_damage += value;
			}
			ItemSuffix::IncreasedFireDamage =>
			{
				increases.fire_damage += value;
			}
			ItemSuffix::IncreasedLightningDamage =>
			{
				increases.lightning_damage += value;
			}
			ItemSuffix::LifeLeech =>
			{
				adds.life_leech += value;
			}
			ItemSuffix::ManaLeech =>
			{
				adds.mana_leech += value;
			}
			ItemSuffix::Duration =>
			{
				increases.skill_duration += value;
			}
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Rarity
{
	Normal,
	Magic,
	Rare,
	Unique,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Item
{
	pub name: Vec<String>,
	pub appearance: Appearance,
	pub rarity: Rarity,
	pub prefixes: Vec<(ItemPrefix, i32, f32)>,
	pub suffixes: Vec<(ItemSuffix, i32, f32)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Inventory
{
	pub slots: [Option<Item>; 9],
}

impl Inventory
{
	pub fn new() -> Self
	{
		Self {
			slots: [None, None, None, None, None, None, None, None, None],
		}
	}
}

pub fn generate_unique(rng: &mut impl Rng) -> Item
{
	match rng.gen_range(0..4)
	{
		1 => Item {
			name: vec!["Polaris".to_string()],
			appearance: Appearance::new("data/ring_cold.cfg"),
			rarity: Rarity::Unique,
			prefixes: vec![
				(ItemPrefix::FreezePropagate, 1, 0.),
				(ItemPrefix::ChanceToFreeze, 25, 0.),
			],
			suffixes: vec![],
		},
		2 => Item {
			name: vec!["Rageheart".to_string()],
			appearance: Appearance::new("data/ring_fire.cfg"),
			rarity: Rarity::Unique,
			prefixes: vec![
				(ItemPrefix::IgnitePropagate, 1, 0.),
				(ItemPrefix::ChanceToIgnite, 25, 0.),
			],
			suffixes: vec![],
		},
		3 => Item {
			name: vec!["Tesla Coil".to_string()],
			appearance: Appearance::new("data/ring_lightning.cfg"),
			rarity: Rarity::Unique,
			prefixes: vec![
				(ItemPrefix::ShockPropagate, 1, 0.),
				(ItemPrefix::ChanceToShock, 25, 0.),
			],
			suffixes: vec![],
		},
		_ => Item {
			name: vec!["Uncontrollable".to_string(), "Hate".to_string()],
			appearance: Appearance::new("data/ring_explode.cfg"),
			rarity: Rarity::Unique,
			prefixes: vec![(ItemPrefix::ExplodeOnDeath, 1, 0.)],
			suffixes: vec![],
		},
	}
}

pub fn generate_item(kind: ItemKind, crystal_level: i32, level: i32, rng: &mut impl Rng) -> Item
{
	let rarity_weights = match crystal_level
	{
		0 => (50, 5, 1),
		1 => (40, 5, 1),
		2 => (30, 5, 1),
		3 => (20, 50, 2),
		4 => (100, 50, 5),
		5 => (100, 50, 7),
		6 => (100, 50, 8),
		7 => (100, 50, 9),
		_ => unreachable!(),
	};

	let rarity = [
		(Rarity::Magic, rarity_weights.0),
		(Rarity::Rare, rarity_weights.1),
		(Rarity::Unique, rarity_weights.2),
	]
	.choose_weighted(rng, |&(_, w)| w)
	.unwrap()
	.0;

	if rarity == Rarity::Unique
	{
		return generate_unique(rng);
	}

	let red_prefix_weights = [
		(ItemPrefix::Life, 1000),
		(ItemPrefix::LifeRegen, 1000),
		(ItemPrefix::AddedPhysicalDamage, 500),
		(ItemPrefix::AddedColdDamage, 50),
		(ItemPrefix::AddedFireDamage, 1000),
		(ItemPrefix::AddedLightningDamage, 50),
		(ItemPrefix::CriticalChance, 50),
		(ItemPrefix::ChanceToFreeze, 10),
		(ItemPrefix::ChanceToIgnite, 50),
		(ItemPrefix::ChanceToShock, 10),
		(ItemPrefix::Mana, 50),
		(ItemPrefix::ManaRegen, 50),
		(ItemPrefix::AreaOfEffect, 500),
		(ItemPrefix::CastSpeed, 50),
	];

	let green_prefix_weights = [
		(ItemPrefix::Life, 50),
		(ItemPrefix::LifeRegen, 50),
		(ItemPrefix::AddedPhysicalDamage, 500),
		(ItemPrefix::AddedColdDamage, 50),
		(ItemPrefix::AddedFireDamage, 50),
		(ItemPrefix::AddedLightningDamage, 1000),
		(ItemPrefix::CriticalChance, 500),
		(ItemPrefix::ChanceToFreeze, 10),
		(ItemPrefix::ChanceToIgnite, 10),
		(ItemPrefix::ChanceToShock, 50),
		(ItemPrefix::Mana, 50),
		(ItemPrefix::ManaRegen, 50),
		(ItemPrefix::AreaOfEffect, 50),
		(ItemPrefix::CastSpeed, 500),
	];

	let blue_prefix_weights = [
		(ItemPrefix::Life, 50),
		(ItemPrefix::LifeRegen, 50),
		(ItemPrefix::AddedPhysicalDamage, 500),
		(ItemPrefix::AddedColdDamage, 1000),
		(ItemPrefix::AddedFireDamage, 50),
		(ItemPrefix::AddedLightningDamage, 50),
		(ItemPrefix::CriticalChance, 50),
		(ItemPrefix::ChanceToFreeze, 50),
		(ItemPrefix::ChanceToIgnite, 10),
		(ItemPrefix::ChanceToShock, 10),
		(ItemPrefix::Mana, 1000),
		(ItemPrefix::ManaRegen, 1000),
		(ItemPrefix::AreaOfEffect, 50),
		(ItemPrefix::CastSpeed, 50),
	];

	let red_suffix_weights = [
		(ItemSuffix::Armour, 50),
		(ItemSuffix::PhysicalResistance, 50),
		(ItemSuffix::ColdResistance, 500),
		(ItemSuffix::FireResistance, 1000),
		(ItemSuffix::LightningResistance, 500),
		(ItemSuffix::CriticalMultiplier, 100),
		(ItemSuffix::IncreasedPhysicalDamage, 500),
		(ItemSuffix::IncreasedColdDamage, 500),
		(ItemSuffix::IncreasedFireDamage, 1000),
		(ItemSuffix::IncreasedLightningDamage, 500),
		(ItemSuffix::LifeLeech, 200),
		(ItemSuffix::ManaLeech, 50),
		(ItemSuffix::Duration, 50),
	];

	let green_suffix_weights = [
		(ItemSuffix::Armour, 50),
		(ItemSuffix::PhysicalResistance, 50),
		(ItemSuffix::ColdResistance, 500),
		(ItemSuffix::FireResistance, 500),
		(ItemSuffix::LightningResistance, 1000),
		(ItemSuffix::CriticalMultiplier, 500),
		(ItemSuffix::IncreasedPhysicalDamage, 500),
		(ItemSuffix::IncreasedColdDamage, 500),
		(ItemSuffix::IncreasedFireDamage, 500),
		(ItemSuffix::IncreasedLightningDamage, 1000),
		(ItemSuffix::LifeLeech, 50),
		(ItemSuffix::ManaLeech, 50),
		(ItemSuffix::Duration, 500),
	];

	let blue_suffix_weights = [
		(ItemSuffix::Armour, 1500),
		(ItemSuffix::PhysicalResistance, 100),
		(ItemSuffix::ColdResistance, 1000),
		(ItemSuffix::FireResistance, 500),
		(ItemSuffix::LightningResistance, 500),
		(ItemSuffix::CriticalMultiplier, 100),
		(ItemSuffix::IncreasedPhysicalDamage, 500),
		(ItemSuffix::IncreasedColdDamage, 1000),
		(ItemSuffix::IncreasedFireDamage, 500),
		(ItemSuffix::IncreasedLightningDamage, 500),
		(ItemSuffix::LifeLeech, 50),
		(ItemSuffix::ManaLeech, 200),
		(ItemSuffix::Duration, 50),
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

	let (num_affixes, min_affixes) = match rarity
	{
		Rarity::Magic => (1, 1),
		Rarity::Rare => (3, 3),
		_ => unreachable!(),
	};

	let mut num_prefixes;
	let mut num_suffixes;
	loop
	{
		num_prefixes = rng.gen_range(0..=num_affixes);
		num_suffixes = rng.gen_range(0..=num_affixes);
		if num_prefixes + num_suffixes >= min_affixes
		{
			break;
		}
	}
	let mut prefixes: Vec<(_, i32, f32)> = vec![];
	for _ in 0..num_prefixes
	{
		loop
		{
			let prefix = prefix_weights.choose_weighted(rng, |&(_, w)| w).unwrap().0;
			if prefixes.iter().find(|p| p.0 == prefix).is_none()
			{
				prefixes.push((
					prefix,
					rng.gen_range(level / 2..=level),
					rng.gen_range(0.0..1.0f32),
				));
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
				suffixes.push((
					suffix,
					rng.gen_range(level / 2..=level),
					rng.gen_range(0.0..1.0f32),
				));
				break;
			}
		}
	}

	let name = match rarity
	{
		Rarity::Normal => unreachable!(),
		Rarity::Magic =>
		{
			make_magic_name(kind, prefixes.first().copied(), suffixes.first().copied())
		}
		Rarity::Rare => make_rare_name(rng),
		Rarity::Unique => unreachable!(),
	};

	prefixes.sort_by_key(|a| a.0);
	suffixes.sort_by_key(|a| a.0);

	let appearance = match kind
	{
		ItemKind::Red => "data/ring_red.cfg",
		ItemKind::Green => "data/ring_yellow.cfg",
		ItemKind::Blue => "data/ring_blue.cfg",
	};
	let appearance = Appearance::new(appearance);
	let item = Item {
		name: name,
		rarity: rarity,
		appearance: appearance,
		prefixes: prefixes,
		suffixes: suffixes,
	};
	//dbg!(&item);
	item
}

fn make_magic_name(
	kind: ItemKind, prefix: Option<(ItemPrefix, i32, f32)>, suffix: Option<(ItemSuffix, i32, f32)>,
) -> Vec<String>
{
	let prefix = prefix.map(|(a, _, _)| a.to_str()).unwrap_or("");
	let suffix = suffix.map(|(a, _, _)| a.to_str()).unwrap_or("");
	vec![
		prefix.to_string(),
		kind.to_str().to_string(),
		suffix.to_string(),
	]
}

fn make_rare_name(rng: &mut impl Rng) -> Vec<String>
{
	let prefix = [
		"Empyrean",
		"Crazed",
		"Foul",
		"Colossal",
		"Thin",
		"Steel",
		"Adamantine",
		"Golden",
		"Shadow",
		"Night",
		"Sun",
		"Indominable",
		"Keen",
		"Dark",
		"Joy",
		"Jolly",
		"Blade",
		"",
		"",
		"",
		"",
	]
	.choose(rng)
	.unwrap();

	let nouns = [
		"Hoop", "Whorl", "Loop", "Curl", "Circle", "Ellipse", "Gape", "Round", "Core", "Center",
		"Border", "Proposal", "Present", "Reward", "Blade", "Spiral", "Point",
	];
	let noun = if rng.gen_bool(0.6)
	{
		nouns.choose(rng).unwrap().to_string()
	}
	else
	{
		let prefix = [
			"Rake",
			"Blade",
			"Spike",
			"Hate",
			"Love",
			"Green",
			"Blue",
			"Red",
			"Violet",
			"Alabaster",
		]
		.choose(rng)
		.unwrap();
		format!("{}{}", prefix, nouns.choose(rng).unwrap())
	};
	let suffix = if rng.gen_bool(0.7)
	{
		[
			"of the Stars",
			"of the Night",
			"of Death",
			"of Life",
			"of Presents",
			"of Beauty",
			"of Hate",
			"of Lust",
			"of Filth",
			"of Sloth",
			"of Blades",
			"",
			"",
			"",
			"",
		]
		.choose(rng)
		.unwrap()
		.to_string()
	}
	else
	{
		let noun = nouns.choose(rng).unwrap();
		format!("the {}", noun)
	};
	vec![prefix.to_string(), noun, suffix]
}

#[derive(Debug, Clone)]
pub struct Waypoints
{
	pub waypoints: Vec<(Point2<f32>, f64)>,
	pub time_to_move: f64,
	pub cur_idx: usize,
}

impl Waypoints
{
	pub fn new(waypoints: Vec<(Point2<f32>, f64)>) -> Self
	{
		Self {
			waypoints: waypoints,
			time_to_move: 0.,
			cur_idx: 0,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Exit;

#[derive(Debug, Copy, Clone)]
pub struct Light
{
	pub color: Color,
	pub offt_y: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct DamageSprites
{
	pub arrow: &'static str,
	pub hit: &'static str,
	pub color: Color,
	pub sound: &'static str,
}

pub fn damage_sprites(values: &StatValues, rarity: Rarity) -> DamageSprites
{
	let damage_vals = [
		values.fire_damage as i32,
		values.cold_damage as i32,
		values.lightning_damage as i32,
	];
	let damage_idx = if damage_vals.iter().sum::<i32>() == 0
	{
		let propagate_vals = [
			(values.ignite_propagate_value.effect * 100.) as i32,
			(values.freeze_propagate_value * 100.) as i32,
			(values.shock_propagate_value.effect * 100.) as i32,
		];
		if propagate_vals.iter().sum::<i32>() == 0
		{
			3
		}
		else
		{
			propagate_vals
				.iter()
				.enumerate()
				.max_by_key(|(_, &v)| v)
				.unwrap()
				.0
		}
	}
	else
	{
		damage_vals
			.iter()
			.enumerate()
			.max_by_key(|(_, &v)| v)
			.unwrap()
			.0
	};

	let f = match rarity
	{
		Rarity::Normal => 0.5,
		Rarity::Magic => 0.75,
		Rarity::Rare => 1.,
		Rarity::Unique => 1.,
	};

	let (arrow, hit, color, sound) = [
		(
			"data/fireball.cfg",
			"data/fire_hit.cfg",
			Color::from_rgb_f(f, f, 0.),
			"data/hit_fire.ogg",
		),
		(
			"data/arrow_cold.cfg",
			"data/cold_hit.cfg",
			Color::from_rgb_f(0., 0., f),
			"data/hit_cold.ogg",
		),
		(
			"data/arrow_lightning.cfg",
			"data/lightning_hit.cfg",
			Color::from_rgb_f(0.5 * f, 0.5 * f, f),
			"data/hit_lightning.ogg",
		),
		(
			"data/arrow_normal.cfg",
			"data/normal_hit.cfg",
			Color::from_rgb_f(f, f, f),
			"data/hit_normal.ogg",
		),
	][damage_idx];

	DamageSprites {
		arrow: arrow,
		hit: hit,
		color: color,
		sound: sound,
	}
}
