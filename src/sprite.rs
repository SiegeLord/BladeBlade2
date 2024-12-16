use crate::error::Result;
use crate::{atlas, game_state, palette, utils};
use allegro::*;
use na::Point2;
use nalgebra as na;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct AnimationDesc
{
	frames: Vec<i32>,
	#[serde(default)]
	frame_ms: Vec<f64>,
	#[serde(default)]
	active_frame: i32,
}

fn default_false() -> bool
{
	false
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct SpriteDesc
{
	bitmap: String,
	width: i32,
	height: i32,
	#[serde(default)]
	center_offt_x: i32,
	#[serde(default)]
	center_offt_y: i32,
	#[serde(default)]
	animations: HashMap<String, AnimationDesc>,
	#[serde(default)]
	palettes: Vec<String>,
}

struct Animation
{
	frames: Vec<atlas::AtlasBitmap>,
	duration_ms: f64,
}

pub struct Sprite
{
	desc: SpriteDesc,
	animations: HashMap<String, Animation>,
}

impl Sprite
{
	pub fn load(
		filename: &str, core: &Core, atlas: &mut atlas::Atlas, palettes: &mut palette::PaletteList,
	) -> Result<Self>
	{
		let mut desc: SpriteDesc = utils::load_config(filename)?;

		let bitmap = if !desc.palettes.is_empty()
		{
			for palette_name in &desc.palettes
			{
				palettes.add_palette(&core, palette_name)?;
			}
			utils::load_bitmap_indexed(&core, &desc.bitmap)?
		}
		else
		{
			utils::load_bitmap(&core, &desc.bitmap)?
		};

		let num_frames_y = bitmap.get_height() / desc.height;
		let num_frames_x = bitmap.get_width() / desc.width;
		let num_frames = num_frames_x * num_frames_y;
		let mut frames = Vec::with_capacity(num_frames as usize);
		for y in 0..num_frames_y
		{
			for x in 0..num_frames_x
			{
				frames.push(
					atlas.insert(
						&core,
						&*bitmap
							.create_sub_bitmap(
								x * desc.width,
								y * desc.height,
								desc.width,
								desc.height,
							)
							.map_err(|_| "Couldn't create sub-bitmap?".to_string())?
							.upgrade()
							.unwrap(),
					)?,
				)
			}
		}

		desc.animations.insert(
			"Default".to_string(),
			AnimationDesc {
				frames: (0..frames.len()).map(|i| i as i32 + 1).collect(),
				frame_ms: vec![],
				active_frame: 0,
			},
		);

		let mut animations = HashMap::new();
		for (name, animation_desc) in &mut desc.animations
		{
			if animation_desc.frame_ms.is_empty()
			{
				animation_desc.frame_ms.push(100.);
			}
			while animation_desc.frame_ms.len() < animation_desc.frames.len()
			{
				animation_desc
					.frame_ms
					.push(*animation_desc.frame_ms.last().unwrap());
			}
			let animation = Animation {
				frames: animation_desc
					.frames
					.iter()
					.map(|&i| frames[(i - 1) as usize].clone())
					.collect(),
				duration_ms: animation_desc.frame_ms.iter().sum(),
			};
			animations.insert(name.to_string(), animation);
		}

		Ok(Sprite {
			desc: desc,
			animations: animations,
		})
	}

	pub fn get_palettes(&self) -> &[String]
	{
		&self.desc.palettes
	}

	pub fn draw(
		&self, pos: Point2<f32>, animation_state: &AnimationState, state: &game_state::GameState,
	)
	{
		self.draw_frame(
			pos,
			&animation_state.animation_name,
			animation_state.frame_idx,
			state,
		);
	}

	pub fn draw_frame(
		&self, pos: Point2<f32>, animation_name: &str, frame_idx: i32,
		state: &game_state::GameState,
	)
	{
		let w = self.desc.width as f32;
		let h = self.desc.height as f32;
		let animation = &self.animations[animation_name];
		let atlas_bmp = &animation.frames[frame_idx as usize];

		state.core.draw_bitmap_region(
			&state.atlas.pages[atlas_bmp.page].bitmap,
			atlas_bmp.start.x,
			atlas_bmp.start.y,
			w,
			h,
			pos.x.floor() - w / 2. - self.desc.center_offt_x as f32,
			pos.y.floor() - h / 2. - self.desc.center_offt_y as f32,
			Flag::zero(),
		);
	}

	pub fn advance_state(&self, state: &mut AnimationState, amount: f64)
	{
		if state.animation_name != state.new_animation_name
		{
			state.animation_name = state.new_animation_name.clone();
			state.frame_idx = 0;
			state.num_activations = 0;
		}
		let animation_desc = &self.desc.animations[&state.animation_name];
		state.frame_progress += amount * 1000.;
		while state.frame_progress > animation_desc.frame_ms[state.frame_idx as usize]
		{
			state.frame_progress -= animation_desc.frame_ms[state.frame_idx as usize];
			state.frame_idx = (state.frame_idx + 1) % animation_desc.frames.len() as i32;
			if state.frame_idx == animation_desc.active_frame
			{
				state.num_activations += 1;
			}
		}
	}
}

#[derive(Debug, Clone)]
pub struct AnimationState
{
	animation_name: String,
	new_animation_name: String,
	frame_progress: f64,
	frame_idx: i32,
	num_activations: i32,
}

impl AnimationState
{
	pub fn new(animation_name: &str) -> Self
	{
		Self {
			animation_name: animation_name.to_string(),
			new_animation_name: animation_name.to_string(),
			frame_progress: 0.,
			frame_idx: 0,
			num_activations: 0,
		}
	}

	pub fn set_new_animation(&mut self, animation_name: impl Into<String>)
	{
		self.new_animation_name = animation_name.into();
	}

	pub fn drain_activations(&mut self) -> i32
	{
		let res = self.num_activations;
		self.num_activations = 0;
		res
	}
}
