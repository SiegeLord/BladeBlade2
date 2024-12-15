use crate::sprite;
use allegro::*;
use na::{Point2, Vector2};
use nalgebra as na;
use rand::prelude::*;

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
pub struct Drawable
{
	pub sprite: String,
	pub palette: Option<String>,
	pub animation_name: String,
	pub animation_start: f64,
	pub animation_speed: f32,
}

impl Drawable
{
	pub fn new(sprite: impl Into<String>) -> Self
	{
		Self {
			sprite: sprite.into(),
			palette: None,
			animation_name: "Default".to_string(),
			animation_start: 0.,
			animation_speed: 1.,
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum CollisionClass
{
	Big,
	Small,
}

impl CollisionClass
{
	pub fn collides_with(&self, other: Self) -> bool
	{
		match (self, other)
		{
			(CollisionClass::Big, CollisionClass::Big) => true,
			(CollisionClass::Big, CollisionClass::Small) => true,
			(CollisionClass::Small, CollisionClass::Big) => true,
			(CollisionClass::Small, CollisionClass::Small) => false,
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
