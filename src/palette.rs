use crate::error::Result;
use crate::utils;
use allegro::*;
use allegro_sys::*;
use std::collections::HashMap;

pub struct PaletteList
{
	num_palettes: i32,
	pub palette_bitmap: Bitmap,
	palette_registry: HashMap<String, i32>,
}

impl PaletteList
{
	pub fn new(core: &Core) -> Self
	{
		Self {
			palette_bitmap: Bitmap::new(&core, 256, 256).unwrap(),
			num_palettes: 0,
			palette_registry: HashMap::new(),
		}
	}

	pub fn add_palette(&mut self, core: &Core, filename: &str) -> Result<()>
	{
		let old_flags = core.get_new_bitmap_flags();
		core.set_new_bitmap_flags(MEMORY_BITMAP);
		let palette_bitmap = utils::load_bitmap(core, filename)?;
		core.set_new_bitmap_flags(old_flags);

		unsafe {
			al_lock_bitmap(
				palette_bitmap.get_allegro_bitmap(),
				ALLEGRO_PIXEL_FORMAT_ANY as i32,
				ALLEGRO_LOCK_READONLY as i32,
			);
			al_lock_bitmap(
				self.palette_bitmap.get_allegro_bitmap(),
				ALLEGRO_PIXEL_FORMAT_ANY as i32,
				ALLEGRO_LOCK_READWRITE as i32,
			);
		}

		core.set_target_bitmap(Some(&self.palette_bitmap));

		let mut target_x = 0;
		for y in 0..palette_bitmap.get_height()
		{
			for x in 0..palette_bitmap.get_width()
			{
				let color = palette_bitmap.get_pixel(x, y);
				core.put_pixel(target_x, self.num_palettes, color);
				target_x += 1;
			}
		}

		unsafe {
			al_unlock_bitmap(palette_bitmap.get_allegro_bitmap());
			al_unlock_bitmap(self.palette_bitmap.get_allegro_bitmap());
		}

		self.palette_registry.insert(filename.to_string(), self.num_palettes);
        self.num_palettes += 1;

		Ok(())
	}

	pub fn get_palette_index(&self, filename: &str) -> Result<i32>
    {
        self.palette_registry.get(filename).map(|&v| v).ok_or(format!("Couldn't find palette {}", filename).into())
    }
}
