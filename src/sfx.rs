use crate::error::Result;
use crate::utils;
use nalgebra::{Point2, Vector2};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use allegro::*;
use allegro_acodec::*;
use allegro_audio::*;

use rand::prelude::*;

const MAX_INSTANCES: usize = 10;
const FADEOUT_TIME: f64 = 0.1;

pub struct Sfx
{
	audio: AudioAddon,
	acodec: AcodecAddon,
	sink: Sink,
	music_stream: Option<AudioStream>,
	music: (String, f32),
	next_music: Option<(String, f32)>,
	time_to_next_music: f64,
	music_fade_factor: f32,
	sample_instances: HashMap<String, Vec<SampleInstance>>,
	exclusive_sounds: Vec<String>,
	exclusive_instance: Option<SampleInstance>,
	sfx_volume: f32,
	music_volume: f32,

	samples: HashMap<String, Sample>,
}

impl Sfx
{
	pub fn new(sfx_volume: f32, music_volume: f32, core: &Core) -> Result<Sfx>
	{
		let audio = AudioAddon::init(&core)?;
		let acodec = AcodecAddon::init(&audio)?;
		let sink = Sink::new(&audio).map_err(|_| "Couldn't create audio sink".to_string())?;

		let mut sfx = Sfx {
			sfx_volume: 0.,
			music_volume: 0.,
			audio: audio,
			acodec: acodec,
			sink: sink,
			sample_instances: HashMap::new(),
			music_stream: None,
			exclusive_instance: None,
			exclusive_sounds: vec![],
			samples: HashMap::new(),
			music: ("".into(), 1.0),
			time_to_next_music: 0.,
			music_fade_factor: 1.0,
			next_music: None,
		};
		sfx.set_sfx_volume(sfx_volume);
		sfx.set_music_volume(music_volume);

		Ok(sfx)
	}

	pub fn cache_sample<'l>(&'l mut self, name: &str) -> Result<&'l Sample>
	{
		Ok(match self.samples.entry(name.to_string())
		{
			Entry::Occupied(o) => o.into_mut(),
			Entry::Vacant(v) => v.insert(utils::load_sample(&self.audio, name)?),
		})
	}

	pub fn get_sample<'l>(&'l self, name: &str) -> Option<&'l Sample>
	{
		self.samples.get(name)
	}

	pub fn update_sounds(&mut self, core: &Core) -> Result<()>
	{
		for instances in self.sample_instances.values_mut()
		{
			instances.retain(|s| s.get_playing().unwrap());
		}
		if self.next_music.is_some()
		{
			if core.get_time() > self.time_to_next_music
			{
				self.music = self.next_music.take().unwrap();
				self.start_music()?;
				self.music_fade_factor = 1.;
			}
			else
			{
				self.music_fade_factor =
					((self.time_to_next_music - core.get_time()) / FADEOUT_TIME) as f32;
				self.set_music_volume(self.music_volume);
			}
		}
		if let Some(ref stream) = self.music_stream
		{
			if !stream.get_playing()
			{
				self.start_music()?;
			}
		}

		if !self.exclusive_sounds.is_empty()
		{
			let mut play_next_sound = true;
			if let Some(exclusive_instance) = &self.exclusive_instance
			{
				play_next_sound = !exclusive_instance.get_playing().unwrap();
			}
			if play_next_sound
			{
				let name = self.exclusive_sounds.pop().unwrap();
				self.cache_sample(&name)?;
				let sample = self.samples.get(&name).unwrap();
				let instance = self
					.sink
					.play_sample(
						sample,
						self.sfx_volume,
						None,
						thread_rng().gen_range(0.9..1.1),
						Playmode::Once,
					)
					.map_err(|_| "Couldn't play sound".to_string())?;
				self.exclusive_instance = Some(instance);
			}
		}

		Ok(())
	}

	fn add_sample_instance(&mut self, name: &str, instance: SampleInstance)
	{
		match self.sample_instances.entry(name.to_string())
		{
			Entry::Occupied(o) =>
			{
				let instances = o.into_mut();

				if instances.len() < MAX_INSTANCES
				{
					instances.push(instance);
				}
			}
			Entry::Vacant(v) =>
			{
				v.insert(vec![instance]);
			}
		}
	}

	pub fn play_sound_with_pitch(&mut self, name: &str, pitch: f32) -> Result<()>
	{
		self.cache_sample(name)?;
		let sample = self.samples.get(name).unwrap();
		let instance = self
			.sink
			.play_sample(
				sample,
				self.sfx_volume,
				None,
				thread_rng().gen_range(0.9..1.1) * pitch,
				Playmode::Once,
			)
			.map_err(|_| "Couldn't play sound".to_string())?;
		self.add_sample_instance(name, instance);
		Ok(())
	}

	pub fn play_sound(&mut self, name: &str) -> Result<()>
	{
		self.cache_sample(name)?;
		let sample = self.samples.get(name).unwrap();
		let instance = self
			.sink
			.play_sample(
				sample,
				self.sfx_volume,
				None,
				thread_rng().gen_range(0.9..1.1),
				Playmode::Once,
			)
			.map_err(|_| "Couldn't play sound".to_string())?;
		self.add_sample_instance(name, instance);
		Ok(())
	}

	pub fn play_continuous_sound(&mut self, name: &str, volume: f32) -> Result<SampleInstance>
	{
		self.cache_sample(name)?;
		let sample = self.samples.get(name).unwrap();
		let instance = self
			.sink
			.play_sample(sample, self.sfx_volume * volume, None, 1., Playmode::Loop)
			.map_err(|_| "Couldn't play sound".to_string())?;
		Ok(instance)
	}

	pub fn play_positional_sound(
		&mut self, name: &str, sound_pos: Point2<f32>, camera_pos: Point2<f32>, volume: f32,
	) -> Result<()>
	{
		self.cache_sample(name)?;

		let sample = self.samples.get(name).unwrap();

		let dist_sq = (sound_pos - camera_pos).norm_squared();
		let base_dist = 100.;
		let volume = self.sfx_volume
			* utils::clamp(
				self.sfx_volume * volume * base_dist * base_dist / dist_sq,
				0.,
				1.,
			);
		//println!("volume: {}", volume);
		let diff = sound_pos - camera_pos;
		let pan = diff.x / (diff.x.powf(2.) + 32.0_f32.powf(2.)).sqrt();

		let instance = self
			.sink
			.play_sample(
				sample,
				volume,
				Some(pan),
				thread_rng().gen_range(0.9..1.1),
				Playmode::Once,
			)
			.map_err(|_| "Couldn't play sound".to_string())?;
		self.add_sample_instance(name, instance);
		Ok(())
	}

	pub fn play_exclusive_sound(&mut self, name: &str) -> Result<()>
	{
		self.exclusive_sounds.insert(0, name.to_string());
		Ok(())
	}

	pub fn play_music(&mut self, music: &str, music_volume_factor: f32, core: &Core)
	{
		self.next_music = Some((music.to_string(), music_volume_factor));
		self.time_to_next_music = core.get_time() + FADEOUT_TIME;
	}

	fn start_music(&mut self) -> Result<()>
	{
		let mut new_stream = AudioStream::load(&self.audio, &self.music.0)
			.map_err(|_| format!("Couldn't load {}", self.music.0))?;
		new_stream.attach(&mut self.sink).unwrap();
		new_stream.set_playmode(Playmode::Loop).unwrap();
		new_stream
			.set_gain(self.music_volume * self.music.1)
			.unwrap();
		self.music_stream = Some(new_stream);
		Ok(())
	}

	pub fn set_music_volume(&mut self, new_volume: f32)
	{
		self.music_volume = new_volume;
		if let Some(stream) = self.music_stream.as_mut()
		{
			stream
				.set_gain(self.music_volume * self.music.1 * self.music_fade_factor)
				.unwrap();
		}
	}

	pub fn set_sfx_volume(&mut self, new_volume: f32)
	{
		self.sfx_volume = new_volume;
	}

	pub fn fade_out(&mut self, core: &Core)
	{
		let mut t = 0.;
		let dt = 0.01;
		while t < FADEOUT_TIME
		{
			self.sink.set_gain((1. - t / FADEOUT_TIME) as f32).unwrap();
			core.rest(dt);
			t += dt;
		}
	}
}
