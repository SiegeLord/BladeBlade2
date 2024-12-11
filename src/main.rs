#![feature(float_next_up_down)]
#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(dead_code)]

mod astar;
mod atlas;
mod components;
mod controls;
mod error;
mod game;
mod game_state;
mod menu;
mod sfx;
mod sprite;
mod ui;
mod utils;

use crate::error::Result;
use allegro::*;
use allegro_dialog::*;
use allegro_sys::*;
use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::rc::Rc;

enum Screen
{
	Game(game::Game),
	Menu(menu::Menu),
}

fn real_main() -> Result<()>
{
	let mut state = game_state::GameState::new()?;

	let mut flags = OPENGL | RESIZABLE | PROGRAMMABLE_PIPELINE;

	if state.options.fullscreen
	{
		flags = flags | FULLSCREEN_WINDOW;
	}
	state.core.set_new_display_flags(flags);

	if state.options.vsync_method == 1
	{
		state.core.set_new_display_option(
			DisplayOption::Vsync,
			1,
			DisplayOptionImportance::Suggest,
		);
	}
	let mut display = Display::new(&state.core, state.options.width, state.options.height)
		.map_err(|_| "Couldn't create display".to_string())?;

	let shader = utils::load_shader(&mut display, "data/basic")?;
	let scale_shader = utils::load_shader(&mut display, "data/scale")?;
	state.resize_display(&display)?;

	let timer = Timer::new(&state.core, utils::DT as f64)
		.map_err(|_| "Couldn't create timer".to_string())?;

	let queue =
		EventQueue::new(&state.core).map_err(|_| "Couldn't create event queue".to_string())?;
	queue.register_event_source(display.get_event_source());
	queue.register_event_source(
		state
			.core
			.get_keyboard_event_source()
			.expect("Couldn't get keyboard"),
	);
	queue.register_event_source(
		state
			.core
			.get_mouse_event_source()
			.expect("Couldn't get mouse"),
	);
	queue.register_event_source(timer.get_event_source());

	let mut quit = false;

	let mut cur_screen = Screen::Menu(menu::Menu::new(&mut state)?);
	//let mut cur_screen = Screen::Game(game::Game::new(
	//	&mut state,
	//)?);

	let mut logics_without_draw = 0;
	let mut old_fullscreen = state.options.fullscreen;
	let mut old_ui_scale = state.options.ui_scale;
	let mut old_frac_scale = state.options.frac_scale;

	let mut prev_frame_start = state.core.get_time();
	let mut logic_end = prev_frame_start;
	let mut frame_count = 0;
	if state.options.grab_mouse
	{
		state.core.grab_mouse(&display).ok();
	}
	//display.show_cursor(false).ok();

	timer.start();
	while !quit
	{
		if queue.is_empty()
		{
			if state.display_width != display.get_width() as f32
				|| state.display_height != display.get_height() as f32
				|| old_ui_scale != state.options.ui_scale
				|| old_frac_scale != state.options.frac_scale
			{
				old_ui_scale = state.options.ui_scale;
				old_frac_scale = state.options.frac_scale;
				state.resize_display(&display)?;
				match &mut cur_screen
				{
					Screen::Game(game) => game.resize(&state),
					Screen::Menu(menu) => menu.resize(&state),
				}
			}

			let frame_start = state.core.get_time();
			state.core.set_target_bitmap(Some(state.buffer1()));
			state.alpha = (frame_start - logic_end) as f32 / utils::DT;

			match &mut cur_screen
			{
				Screen::Game(game) => game.draw(&state)?,
				Screen::Menu(menu) => menu.draw(&state)?,
			}

			if state.options.vsync_method == 2
			{
				state.core.wait_for_vsync().ok();
			}

			state.core.set_target_bitmap(Some(state.buffer2()));

			state
				.core
				.use_shader(Some(&*shader.upgrade().unwrap()))
				.unwrap();

			state
				.core
				.draw_bitmap(state.buffer1(), 0., 0., Flag::zero());

			state.core.set_target_bitmap(Some(display.get_backbuffer()));

			let bw = state.buffer_width() as f32;
			let bh = state.buffer_height() as f32;
			let dw = display.get_width() as f32;
			let dh = display.get_height() as f32;

			state
				.core
				.use_shader(Some(&*scale_shader.upgrade().unwrap()))
				.unwrap();
			state
				.core
				.set_shader_uniform("bitmap_width", &[bw][..])
				.ok();
			state
				.core
				.set_shader_uniform("bitmap_height", &[bh][..])
				.ok();
			state
				.core
				.set_shader_uniform("scale", &[state.draw_scale][..])
				.ok();

			state.core.clear_to_color(Color::from_rgb_f(0., 0., 0.));

			state.core.draw_scaled_bitmap(
				state.buffer2(),
				0.,
				0.,
				bw,
				bh,
				(dw / 2. - bw / 2. * state.draw_scale).floor(),
				(dh / 2. - bh / 2. * state.draw_scale).floor(),
				bw * state.draw_scale,
				bh * state.draw_scale,
				Flag::zero(),
			);

			state.core.flip_display();

			if frame_count == 120
			{
				println!("FPS: {:.2}", 120. / (frame_start - prev_frame_start));
				prev_frame_start = frame_start;
				frame_count = 0;
			}
			frame_count += 1;
			logics_without_draw = 0;
		}

		let event = queue.get_next_event();
		let mut next_screen = match &mut cur_screen
		{
			Screen::Game(game) => game.input(&event, &mut state)?,
			Screen::Menu(menu) => menu.input(&event, &mut state)?,
		};

		match event
		{
			Event::DisplayClose { .. } => quit = true,
			Event::DisplayResize { .. } =>
			{
				display
					.acknowledge_resize()
					.map_err(|_| "Couldn't acknowledge resize".to_string())?;
			}
			Event::DisplaySwitchIn { .. } =>
			{
				if state.options.grab_mouse
				{
					state.core.grab_mouse(&display).ok();
				}
				//display.show_cursor(false).ok();
				state.track_mouse = true;
			}
			Event::DisplaySwitchOut { .. } =>
			{
				if state.options.grab_mouse
				{
					state.core.ungrab_mouse().ok();
				}
				//display.show_cursor(true).ok();
				state.track_mouse = false;
			}
			Event::MouseButtonDown { .. } =>
			{
				if state.options.grab_mouse
				{
					state.core.grab_mouse(&display).ok();
				}
				//display.show_cursor(false).ok();
				state.track_mouse = true;
			}
			Event::TimerTick { .. } =>
			{
				if logics_without_draw > 10
				{
					continue;
				}

				if next_screen.is_none()
				{
					next_screen = match &mut cur_screen
					{
						Screen::Game(game) => game.logic(&mut state)?,
						_ => None,
					}
				}

				if old_fullscreen != state.options.fullscreen
				{
					display.set_flag(FULLSCREEN_WINDOW, state.options.fullscreen);
					old_fullscreen = state.options.fullscreen;
				}

				logics_without_draw += 1;
				state.sfx.update_sounds()?;

				if !state.paused
				{
					state.tick += 1;
				}
				logic_end = state.core.get_time();
			}
			_ => (),
		}

		if let Some(next_screen) = next_screen
		{
			match next_screen
			{
				game_state::NextScreen::Game =>
				{
					cur_screen = Screen::Game(game::Game::new(&mut state)?);
				}
				game_state::NextScreen::Menu =>
				{
					cur_screen = Screen::Menu(menu::Menu::new(&mut state)?);
				}
				game_state::NextScreen::Quit =>
				{
					quit = true;
				}
				_ => panic!("Unknown next screen {:?}", next_screen),
			}
		}
	}
	// To avoid clicks for the final sound.
	state.core.rest(0.25);

	Ok(())
}

allegro_main! {
	use std::panic::catch_unwind;

	match catch_unwind(|| real_main().unwrap())
	{
		Err(e) =>
		{
			let err: String = e
				.downcast_ref::<&'static str>()
				.map(|&e| e.to_owned())
				.or_else(|| e.downcast_ref::<String>().map(|e| e.clone()))
				.unwrap_or("Unknown error!".to_owned());

			let mut lines = vec![];
			for line in err.lines().take(10)
			{
				lines.push(line.to_string());
			}
			show_native_message_box(
				None,
				"Error!",
				"An error has occurred!",
				&lines.join("\n"),
				Some("You make me sad."),
				MESSAGEBOX_ERROR,
			);
		}
		Ok(_) => (),
	}
}
