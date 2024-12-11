use crate::sprite;
use allegro::*;
use na::Point2;
use nalgebra as na;
use rand::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Position
{
	pub pos: Point2<f32>,
	old_pos: Point2<f32>,
}

impl Position
{
	pub fn new(pos: Point2<f32>) -> Self
	{
		Self { pos, old_pos: pos }
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
