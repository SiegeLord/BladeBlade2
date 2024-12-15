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
		&self, pos: Point2<f32>, animation_name: &str, time: f64, speed: f32,
		state: &game_state::GameState,
	)
	{
		// Awkward to do the lookup twice?
		let animation = &self.animations[animation_name];
		let animation_desc = &self.desc.animations[animation_name];
		let time = (time * 1000. * speed as f64) % animation.duration_ms;

		// TODO: Should I make this stateful to avoid the scan?
		let mut frame_idx = 0;
		let mut cur_time = 0.;
		for (i, dt) in animation_desc.frame_ms.iter().enumerate()
		{
			if cur_time + dt > time
			{
				frame_idx = i;
				break;
			}
			cur_time += dt;
		}

		self.draw_frame(pos, animation_name, frame_idx as i32, state)
	}

	pub fn draw_frame(
		&self, pos: Point2<f32>, animation_name: &str, frame_idx: i32,
		state: &game_state::GameState,
	)
	{
		let w = self.desc.width as f32;
		let h = self.desc.height as f32;
		// Awkward to do the lookup three times!!!!?
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
}
