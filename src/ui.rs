use crate::error::Result;
use crate::{components, controls, game_state, utils};

use allegro::*;
use allegro_font::*;
use allegro_sys::*;
use nalgebra::{Matrix4, Point2, Vector2, Vector3};

pub const UNSELECTED: Color = Color::from_rgb_f(0.5, 0.5, 0.9);
pub const LABEL: Color = Color::from_rgb_f(0.8 * 0.5, 0.8 * 0.5, 0.8 * 0.9);
pub const SELECTED: Color = Color::from_rgb_f(1., 1., 1.);

pub const HORIZ_SPACE: f32 = 48.;
pub const VERT_SPACE: f32 = 16.;
pub const BUTTON_WIDTH: f32 = 128.;
pub const BUTTON_HEIGHT: f32 = 16.;
pub const CONTROL_WIDTH: f32 = 80.;

#[derive(Clone, Debug, PartialEq)]
pub enum Action
{
	SelectMe,
	MainMenu,
	Start,
	Quit,
	Back,
	Forward(fn(&mut game_state::GameState) -> Result<SubScreen>),
	ToggleFullscreen,
	ToggleFracScale,
	ChangeInput(controls::Action, usize),
	MouseSensitivity(f32),
	UiScale(f32),
	MusicVolume(f32),
	SfxVolume(f32),
	CameraSpeed(i32),
}

#[derive(Clone)]
struct Button
{
	loc: Point2<f32>,
	size: Vector2<f32>,
	text: String,
	action: Action,
	selected: bool,
}

impl Button
{
	fn new(w: f32, h: f32, text: &str, action: Action) -> Self
	{
		Self {
			loc: Point2::new(0., 0.),
			size: Vector2::new(w, h),
			text: text.into(),
			action: action,
			selected: false,
		}
	}

	fn width(&self) -> f32
	{
		self.size.x
	}

	fn height(&self) -> f32
	{
		self.size.y
	}

	fn draw(&self, state: &game_state::GameState)
	{
		let c_ui = if self.selected { SELECTED } else { UNSELECTED };

		state.core.draw_text(
			state.ui_font(),
			c_ui,
			self.loc.x.round(),
			(self.loc.y - state.ui_font().get_line_height() as f32 / 2.).round(),
			FontAlign::Centre,
			&self.text,
		);
	}

	fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		let s = state.options.ui_scale;
		let start = self.loc - s * self.size / 2.;
		let end = self.loc + s * self.size / 2.;
		match event
		{
			Event::MouseAxes { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					return Some(Action::SelectMe);
				}
			}
			Event::KeyDown { keycode, .. } => match keycode
			{
				KeyCode::Enter | KeyCode::Space =>
				{
					if self.selected
					{
						state.sfx.play_sound("data/ui2.ogg").unwrap();
						return Some(self.action.clone());
					}
				}
				KeyCode::Escape =>
				{
					if self.action == Action::Back
					{
						state.sfx.play_sound("data/ui2.ogg").unwrap();
						return Some(self.action.clone());
					}
				}
				_ => (),
			},
			Event::MouseButtonUp { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					state.sfx.play_sound("data/ui2.ogg").unwrap();
					return Some(self.action.clone());
				}
			}
			_ => (),
		}
		None
	}
}

#[derive(Clone)]
struct Toggle
{
	loc: Point2<f32>,
	size: Vector2<f32>,
	texts: Vec<String>,
	cur_value: usize,
	action_fn: fn(usize) -> Action,
	selected: bool,
}

impl Toggle
{
	fn new(
		w: f32, h: f32, cur_value: usize, texts: Vec<String>, action_fn: fn(usize) -> Action,
	) -> Self
	{
		Self {
			loc: Point2::new(0., 0.),
			size: Vector2::new(w, h),
			texts: texts,
			cur_value: cur_value,
			action_fn: action_fn,
			selected: false,
		}
	}

	fn width(&self) -> f32
	{
		self.size.x
	}

	fn height(&self) -> f32
	{
		self.size.y
	}

	fn draw(&self, state: &game_state::GameState)
	{
		let c_ui = if self.selected { SELECTED } else { UNSELECTED };

		state.core.draw_text(
			state.ui_font(),
			c_ui,
			self.loc.x,
			self.loc.y - state.ui_font().get_line_height() as f32 / 2.,
			FontAlign::Centre,
			&self.texts[self.cur_value],
		);
	}

	fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		let s = state.options.ui_scale;
		let start = self.loc - s * self.size / 2.;
		let end = self.loc + s * self.size / 2.;
		match event
		{
			Event::MouseAxes { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					return Some(Action::SelectMe);
				}
			}
			Event::KeyDown { keycode, .. } => match keycode
			{
				KeyCode::Enter | KeyCode::Space =>
				{
					if self.selected
					{
						return Some(self.trigger(state));
					}
				}
				_ => (),
			},
			Event::MouseButtonUp { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					return Some(self.trigger(state));
				}
			}
			_ => (),
		}
		None
	}

	fn trigger(&mut self, state: &mut game_state::GameState) -> Action
	{
		state.sfx.play_sound("data/ui2.ogg").unwrap();
		self.cur_value = (self.cur_value + 1) % self.texts.len();
		(self.action_fn)(self.cur_value)
	}
}

#[derive(Clone)]
struct Slider
{
	loc: Point2<f32>,
	size: Vector2<f32>,
	cur_pos: f32,
	min_pos: f32,
	max_pos: f32,
	grabbed: bool,
	selected: bool,
	round_to: f32,
	action_fn: fn(f32) -> Action,
}

impl Slider
{
	fn new(
		w: f32, h: f32, cur_pos: f32, min_pos: f32, max_pos: f32, round_to: f32,
		action_fn: fn(f32) -> Action,
	) -> Self
	{
		Self {
			loc: Point2::new(0., 0.),
			size: Vector2::new(w, h),
			cur_pos: cur_pos,
			min_pos: min_pos,
			max_pos: max_pos,
			grabbed: false,
			selected: false,
			round_to: round_to,
			action_fn: action_fn,
		}
	}

	fn width(&self) -> f32
	{
		self.size.x
	}

	fn height(&self) -> f32
	{
		self.size.y
	}

	fn round_cur_pos(&mut self)
	{
		self.cur_pos = (self.cur_pos / self.round_to).round() * self.round_to;
	}

	fn draw(&self, state: &game_state::GameState)
	{
		let s = state.options.ui_scale;
		let c_ui = if self.selected { SELECTED } else { UNSELECTED };

		let w = s * self.width();
		let cursor_x =
			self.loc.x - w / 2. + w * (self.cur_pos - self.min_pos) / (self.max_pos - self.min_pos);
		let start_x = self.loc.x - w / 2.;
		let end_x = self.loc.x + w / 2.;

		let ww = s * HORIZ_SPACE;
		if cursor_x - start_x > ww
		{
			state
				.prim
				.draw_line(start_x, self.loc.y, cursor_x - ww, self.loc.y, c_ui, s * 4.);
		}
		if end_x - cursor_x > ww
		{
			state
				.prim
				.draw_line(cursor_x + ww, self.loc.y, end_x, self.loc.y, c_ui, s * 4.);
		}
		//state.prim.draw_filled_circle(self.loc.x - w / 2. + w * self.cur_pos / self.max_pos, self.loc.y, 8., c_ui);

		let text = format!("{:.2}", self.cur_pos);
		let text = if text.contains('.')
		{
			text.trim_end_matches("0").trim_end_matches(".")
		}
		else
		{
			&text
		};

		state.core.draw_text(
			state.ui_font(),
			c_ui,
			cursor_x.floor(),
			self.loc.y - state.ui_font().get_line_height() as f32 / 2.,
			FontAlign::Centre,
			text,
		);
	}

	fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		let s = state.options.ui_scale;
		let start = self.loc - s * self.size / 2.;
		let end = self.loc + s * self.size / 2.;
		match event
		{
			Event::MouseAxes { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					if self.grabbed
					{
						self.cur_pos = self.min_pos
							+ (x - start.x) / (s * self.width()) * (self.max_pos - self.min_pos);
						self.round_cur_pos();
						return Some((self.action_fn)(self.cur_pos));
					}
					else
					{
						return Some(Action::SelectMe);
					}
				}
			}
			Event::MouseButtonUp { .. } =>
			{
				self.grabbed = false;
			}
			Event::MouseButtonDown { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					state.sfx.play_sound("data/ui2.ogg").unwrap();
					self.grabbed = true;
					self.cur_pos = self.min_pos
						+ (x - start.x) / (s * self.width()) * (self.max_pos - self.min_pos);
					self.round_cur_pos();
					return Some((self.action_fn)(self.cur_pos));
				}
			}
			Event::KeyDown { keycode, .. } =>
			{
				let increment = self.round_to;
				if self.selected
				{
					match keycode
					{
						KeyCode::Left =>
						{
							if self.cur_pos > self.min_pos
							{
								state.sfx.play_sound("data/ui2.ogg").unwrap();
								self.cur_pos = utils::max(self.min_pos, self.cur_pos - increment);
								self.round_cur_pos();
								return Some((self.action_fn)(self.cur_pos));
							}
						}
						KeyCode::Right =>
						{
							if self.cur_pos < self.max_pos
							{
								state.sfx.play_sound("data/ui2.ogg").unwrap();
								self.cur_pos = utils::min(self.max_pos, self.cur_pos + increment);
								self.round_cur_pos();
								return Some((self.action_fn)(self.cur_pos));
							}
						}
						_ => (),
					}
				}
			}
			_ => (),
		}
		None
	}
}

#[derive(Clone)]
struct Label
{
	loc: Point2<f32>,
	size: Vector2<f32>,
	text: String,
}

impl Label
{
	fn new(w: f32, h: f32, text: &str) -> Self
	{
		Self {
			loc: Point2::new(0., 0.),
			size: Vector2::new(w, h),
			text: text.into(),
		}
	}

	fn width(&self) -> f32
	{
		self.size.x
	}

	fn height(&self) -> f32
	{
		self.size.y
	}

	fn draw(&self, state: &game_state::GameState)
	{
		state.core.draw_text(
			state.ui_font(),
			LABEL,
			self.loc.x,
			self.loc.y - state.ui_font().get_line_height() as f32 / 2.,
			FontAlign::Centre,
			&self.text,
		);
	}

	fn input(&mut self, _state: &mut game_state::GameState, _event: &Event) -> Option<Action>
	{
		None
	}
}

#[derive(Clone)]
enum Widget
{
	Button(Button),
	Label(Label),
	Slider(Slider),
	Toggle(Toggle),
}

impl Widget
{
	fn height(&self) -> f32
	{
		match self
		{
			Widget::Button(w) => w.height(),
			Widget::Label(w) => w.height(),
			Widget::Slider(w) => w.height(),
			Widget::Toggle(w) => w.height(),
		}
	}

	fn width(&self) -> f32
	{
		match self
		{
			Widget::Button(w) => w.width(),
			Widget::Label(w) => w.width(),
			Widget::Slider(w) => w.width(),
			Widget::Toggle(w) => w.width(),
		}
	}

	fn loc(&self) -> Point2<f32>
	{
		match self
		{
			Widget::Button(w) => w.loc,
			Widget::Label(w) => w.loc,
			Widget::Slider(w) => w.loc,
			Widget::Toggle(w) => w.loc,
		}
	}

	fn selectable(&self) -> bool
	{
		match self
		{
			Widget::Button(_) => true,
			Widget::Label(_) => false,
			Widget::Slider(_) => true,
			Widget::Toggle(_) => true,
		}
	}

	fn set_loc(&mut self, loc: Point2<f32>)
	{
		match self
		{
			Widget::Button(ref mut w) => w.loc = loc,
			Widget::Label(ref mut w) => w.loc = loc,
			Widget::Slider(ref mut w) => w.loc = loc,
			Widget::Toggle(ref mut w) => w.loc = loc,
		}
	}

	fn selected(&self) -> bool
	{
		match self
		{
			Widget::Button(w) => w.selected,
			Widget::Label(_) => false,
			Widget::Slider(w) => w.selected,
			Widget::Toggle(w) => w.selected,
		}
	}

	fn set_selected(&mut self, selected: bool)
	{
		match self
		{
			Widget::Button(ref mut w) => w.selected = selected,
			Widget::Label(_) => (),
			Widget::Slider(ref mut w) => w.selected = selected,
			Widget::Toggle(ref mut w) => w.selected = selected,
		}
	}

	fn draw(&self, state: &game_state::GameState)
	{
		match self
		{
			Widget::Button(w) => w.draw(state),
			Widget::Label(w) => w.draw(state),
			Widget::Slider(w) => w.draw(state),
			Widget::Toggle(w) => w.draw(state),
		}
	}

	fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		match self
		{
			Widget::Button(w) => w.input(state, event),
			Widget::Label(w) => w.input(state, event),
			Widget::Slider(w) => w.input(state, event),
			Widget::Toggle(w) => w.input(state, event),
		}
	}
}

struct WidgetList
{
	widgets: Vec<Vec<Widget>>,
	cur_selection: (usize, usize),
	pos: Point2<f32>,
}

impl WidgetList
{
	fn new(widgets: &[&[Widget]]) -> Self
	{
		let mut new_widgets = Vec::with_capacity(widgets.len());
		let mut cur_selection = None;
		for (i, row) in widgets.iter().enumerate()
		{
			let mut new_row = Vec::with_capacity(row.len());
			for (j, w) in row.iter().enumerate()
			{
				if w.selectable() && cur_selection.is_none()
				{
					cur_selection = Some((i, j));
				}
				new_row.push(w.clone());
			}
			new_widgets.push(new_row);
		}

		if let Some((i, j)) = cur_selection
		{
			new_widgets[i][j].set_selected(true);
		}

		Self {
			pos: Point2::new(0., 0.),
			widgets: new_widgets,
			cur_selection: cur_selection.expect("No selectable widgets?"),
		}
	}

	pub fn draw(&self, state: &game_state::GameState)
	{
		for row in &self.widgets
		{
			for w in row
			{
				w.draw(state);
			}
		}
	}

	pub fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		let mut action = None;
		let old_selection = self.cur_selection;
		'got_action: for (i, row) in self.widgets.iter_mut().enumerate()
		{
			for (j, w) in row.iter_mut().enumerate()
			{
				let cur_action = w.input(state, event);
				if cur_action.is_some()
				{
					action = cur_action;
					if self.cur_selection != (i, j) && action == Some(Action::SelectMe)
					{
						state.sfx.play_sound("data/ui1.ogg").unwrap();
					}
					self.cur_selection = (i, j);
					break 'got_action;
				}
			}
		}
		if action.is_none() || action == Some(Action::SelectMe)
		{
			match event
			{
				Event::KeyDown { keycode, .. } => match *keycode
				{
					KeyCode::Up =>
					{
						state.sfx.play_sound("data/ui1.ogg").unwrap();
						'found1: loop
						{
							self.cur_selection.0 = (self.cur_selection.0 + self.widgets.len() - 1)
								% self.widgets.len();
							let row_len = self.widgets[self.cur_selection.0].len();
							if self.cur_selection.1 >= row_len
							{
								self.cur_selection.1 = row_len - 1;
							}
							for _ in 0..row_len
							{
								if self.widgets[self.cur_selection.0][self.cur_selection.1]
									.selectable()
								{
									break 'found1;
								}
								self.cur_selection.1 =
									(self.cur_selection.1 + row_len - 1) % row_len;
							}
						}
					}
					KeyCode::Down =>
					{
						state.sfx.play_sound("data/ui1.ogg").unwrap();
						'found2: loop
						{
							self.cur_selection.0 = (self.cur_selection.0 + self.widgets.len() + 1)
								% self.widgets.len();
							let row_len = self.widgets[self.cur_selection.0].len();
							if self.cur_selection.1 >= row_len
							{
								self.cur_selection.1 = row_len - 1;
							}
							for _ in 0..row_len
							{
								if self.widgets[self.cur_selection.0][self.cur_selection.1]
									.selectable()
								{
									break 'found2;
								}
								self.cur_selection.1 =
									(self.cur_selection.1 + row_len - 1) % row_len;
							}
						}
					}
					KeyCode::Left =>
					{
						state.sfx.play_sound("data/ui1.ogg").unwrap();
						let row_len = self.widgets[self.cur_selection.0].len();
						loop
						{
							self.cur_selection.1 = (self.cur_selection.1 + row_len - 1) % row_len;
							if self.widgets[self.cur_selection.0][self.cur_selection.1].selectable()
							{
								break;
							}
						}
					}
					KeyCode::Right =>
					{
						state.sfx.play_sound("data/ui1.ogg").unwrap();
						let row_len = self.widgets[self.cur_selection.0].len();
						loop
						{
							self.cur_selection.1 = (self.cur_selection.1 + row_len + 1) % row_len;
							if self.widgets[self.cur_selection.0][self.cur_selection.1].selectable()
							{
								break;
							}
						}
					}
					_ => (),
				},
				_ => (),
			}
		}
		self.widgets[old_selection.0][old_selection.1].set_selected(false);
		self.widgets[self.cur_selection.0][self.cur_selection.1].set_selected(true);
		action
	}

	fn resize(&mut self, state: &game_state::GameState)
	{
		let s = state.options.ui_scale;
		let w_space = s * HORIZ_SPACE;
		let h_space = s * VERT_SPACE;
		let cx = self.pos.x;
		let cy = self.pos.y;

		let mut y = 0.;
		let mut cur_selection = None;
		let num_rows = self.widgets.len();
		for (i, row) in self.widgets.iter_mut().enumerate()
		{
			let mut max_height = -f32::INFINITY;
			let mut x = 0.;

			// Place the relative x's, collect max height.
			let num_cols = row.len();
			for (j, w) in row.iter_mut().enumerate()
			{
				if w.selectable() && cur_selection.is_none()
				{
					cur_selection = Some((i, j));
				}
				if j > 0
				{
					x += (w_space + s * w.width()) / 2.;
				}
				let mut loc = w.loc();
				loc.x = x;
				w.set_loc(loc);
				max_height = utils::max(max_height, s * w.height());
				if j + 1 < num_cols
				{
					x += (w_space + s * w.width()) / 2.;
				}
			}

			if i > 0
			{
				y += (h_space + max_height) / 2.;
			}

			// Place the relative y's, shift the x's.
			for w in row.iter_mut()
			{
				let mut loc = w.loc();
				loc.y = y;
				loc.x += cx - x / 2.;
				w.set_loc(loc);
			}

			if i + 1 < num_rows
			{
				y += (h_space + max_height) / 2.;
			}
		}

		// Shift the y's
		for row in self.widgets.iter_mut()
		{
			for w in row.iter_mut()
			{
				let mut loc = w.loc();
				loc.y += cy - y / 2.;
				w.set_loc(loc);
			}
		}
	}
}

pub struct MainMenu
{
	widgets: WidgetList,
}

impl MainMenu
{
	pub fn new(state: &game_state::GameState) -> Self
	{
		let w = BUTTON_WIDTH;
		let h = BUTTON_HEIGHT;

		let widgets = WidgetList::new(&[
			&[Widget::Button(Button::new(w, h, "New Game", Action::Start))],
			&[Widget::Button(Button::new(
				w,
				h,
				"Controls",
				Action::Forward(|s| Ok(SubScreen::ControlsMenu(ControlsMenu::new(s)))),
			))],
			&[Widget::Button(Button::new(
				w,
				h,
				"Options",
				Action::Forward(|s| Ok(SubScreen::OptionsMenu(OptionsMenu::new(s)))),
			))],
			&[Widget::Button(Button::new(w, h, "Quit", Action::Quit))],
		]);
		let mut res = Self { widgets: widgets };
		res.resize(state);
		res
	}

	pub fn draw(&self, state: &game_state::GameState)
	{
		self.widgets.draw(state);

		let sprite = "data/title.cfg";
		let sprite = state
			.get_sprite(sprite)
			.expect(&format!("Could not find sprite: {}", sprite));
		sprite.draw(
			Point2::new(state.buffer_width() / 2., state.buffer_height() / 2. - 125.),
			0,
			Color::from_rgb_f(1., 1., 1.),
			state,
		);
		let lh = state.ui_font().get_line_height() as f32;
		state.core.draw_text(
			state.ui_font(),
			UNSELECTED,
			HORIZ_SPACE,
			state.buffer_height() - lh - VERT_SPACE,
			FontAlign::Left,
			&format!("Version: {}", game_state::VERSION),
		);
	}

	pub fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		self.widgets.input(state, event)
	}

	pub fn resize(&mut self, state: &game_state::GameState)
	{
		let cx = state.buffer_width() / 2.;
		let cy = state.buffer_height() / 2. + 16.;

		self.widgets.pos.x = cx;
		self.widgets.pos.y = cy;
		self.widgets.resize(state);
	}
}

pub struct ControlsMenu
{
	widgets: WidgetList,
	accepting_input: bool,
}

impl ControlsMenu
{
	pub fn new(state: &game_state::GameState) -> Self
	{
		let w = CONTROL_WIDTH;
		let h = BUTTON_HEIGHT;

		let mut widgets = vec![];
		// widgets.push(vec![
		// 	Widget::Label(Label::new(0., 0., w * 1.5, h, "MOUSE SENSITIVITY")),
		// 	Widget::Slider(Slider::new(
		// 		0.,
		// 		0.,
		// 		w,
		// 		h,
		// 		state.controls.get_mouse_sensitivity(),
		// 		0.,
		// 		2.,
		// 		false,
		// 		|i| Action::MouseSensitivity(i),
		// 	)),
		// ]);

		for (&action, &inputs) in state.controls.get_actions_to_inputs()
		{
			let mut row = vec![Widget::Label(Label::new(w, h, &action.to_str()))];
			for i in 0..2
			{
				let input = inputs[i];
				let input_str = input
					.map(|i| i.to_str().to_string())
					.unwrap_or("None".into());
				row.push(Widget::Button(Button::new(
					w,
					h,
					&input_str,
					Action::ChangeInput(action, i),
				)));
			}
			widgets.push(row);
		}
		widgets.push(vec![Widget::Button(Button::new(
			w,
			h,
			"Back",
			Action::Back,
		))]);

		let mut res = Self {
			widgets: WidgetList::new(&widgets.iter().map(|r| &r[..]).collect::<Vec<_>>()),
			accepting_input: false,
		};
		res.resize(state);
		res
	}

	pub fn draw(&self, state: &game_state::GameState)
	{
		self.widgets.draw(state);
	}

	pub fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		let mut action = None;
		let mut options_changed = false;
		if self.accepting_input
		{
			match &mut self.widgets.widgets[self.widgets.cur_selection.0]
				[self.widgets.cur_selection.1]
			{
				Widget::Button(b) =>
				{
					if let Action::ChangeInput(action, index) = b.action
					{
						if let Some(changed) = state.controls.change_action(action, index, event)
						{
							options_changed = changed;
							state.sfx.play_sound("data/ui2.ogg").unwrap();
							self.accepting_input = false;
						}
					}
				}
				_ => (),
			}
		}
		else
		{
			if let allegro::Event::KeyDown {
				keycode: allegro::KeyCode::Delete,
				..
			} = event
			{
				match &mut self.widgets.widgets[self.widgets.cur_selection.0]
					[self.widgets.cur_selection.1]
				{
					Widget::Button(b) =>
					{
						if let Action::ChangeInput(action, index) = b.action
						{
							state.controls.clear_action(action, index);
							options_changed = true;
							state.sfx.play_sound("data/ui2.ogg").unwrap();
						}
					}
					_ => (),
				}
			}
			action = self.widgets.input(state, event);
			match action
			{
				Some(Action::ChangeInput(_, _)) =>
				{
					self.accepting_input = true;
					match &mut self.widgets.widgets[self.widgets.cur_selection.0]
						[self.widgets.cur_selection.1]
					{
						Widget::Button(b) => b.text = "<Input>".into(),
						_ => (),
					}
				}
				Some(Action::MouseSensitivity(ms)) =>
				{
					state.controls.set_mouse_sensitivity(ms);
					options_changed = true;
				}
				Some(Action::Back) =>
				{
					game_state::save_options(&state.core, &state.options).unwrap();
				}
				_ => (),
			}
		}
		if options_changed
		{
			for widget_row in &mut self.widgets.widgets
			{
				for widget in widget_row
				{
					match widget
					{
						Widget::Button(b) =>
						{
							if let Action::ChangeInput(action, index) = b.action
							{
								b.text = state.controls.get_inputs(action).unwrap()[index]
									.map(|a| a.to_str().to_string())
									.unwrap_or("None".into());
							}
						}
						_ => (),
					}
				}
			}
			state.options.controls = state.controls.get_controls().clone();
		}
		action
	}

	pub fn resize(&mut self, state: &game_state::GameState)
	{
		let cx = state.buffer_width() / 2.;
		let cy = state.buffer_height() / 2.;
		self.widgets.pos.x = cx;
		self.widgets.pos.y = cy;
		self.widgets.resize(state);
	}
}

pub struct OptionsMenu
{
	widgets: WidgetList,
}

impl OptionsMenu
{
	pub fn new(state: &game_state::GameState) -> Self
	{
		let w = BUTTON_WIDTH;
		let h = BUTTON_HEIGHT;

		let widgets = [
			vec![
				Widget::Label(Label::new(w, h, "Fullscreen")),
				Widget::Toggle(Toggle::new(
					w,
					h,
					state.options.fullscreen as usize,
					vec!["No".into(), "Yes".into()],
					|_| Action::ToggleFullscreen,
				)),
			],
			vec![
				Widget::Label(Label::new(w, h, "Fractional Scale")),
				Widget::Toggle(Toggle::new(
					w,
					h,
					state.options.frac_scale as usize,
					vec!["No".into(), "Yes".into()],
					|_| Action::ToggleFracScale,
				)),
			],
			vec![
				Widget::Label(Label::new(w, h, "Music")),
				Widget::Slider(Slider::new(
					w,
					h,
					state.options.music_volume,
					0.,
					4.,
					0.1,
					|i| Action::MusicVolume(i),
				)),
			],
			vec![
				Widget::Label(Label::new(w, h, "SFX")),
				Widget::Slider(Slider::new(
					w,
					h,
					state.options.music_volume,
					0.,
					4.,
					0.1,
					|i| Action::SfxVolume(i),
				)),
			],
			vec![
				Widget::Label(Label::new(w, h, "UI Scale")),
				Widget::Slider(Slider::new(
					w,
					h,
					state.options.ui_scale,
					1.,
					4.,
					0.25,
					|i| Action::UiScale(i),
				)),
			],
			vec![
				Widget::Label(Label::new(w, h, "Scroll")),
				Widget::Slider(Slider::new(
					w,
					h,
					state.options.camera_speed as f32,
					1.,
					10.,
					1.,
					|i| Action::CameraSpeed(i as i32),
				)),
			],
			vec![Widget::Button(Button::new(w, h, "Back", Action::Back))],
		];

		let mut res = Self {
			widgets: WidgetList::new(&widgets.iter().map(|r| &r[..]).collect::<Vec<_>>()),
		};
		res.resize(state);
		res
	}

	pub fn draw(&self, state: &game_state::GameState)
	{
		self.widgets.draw(state);
	}

	pub fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		let action = self.widgets.input(state, event);
		if let Some(action) = action
		{
			match action
			{
				Action::ToggleFullscreen =>
				{
					state.options.fullscreen = !state.options.fullscreen;
				}
				Action::ToggleFracScale =>
				{
					state.options.frac_scale = !state.options.frac_scale;
				}
				Action::MusicVolume(v) =>
				{
					state.options.music_volume = v;
					state.sfx.set_music_volume(v);
				}
				Action::CameraSpeed(i) =>
				{
					state.options.camera_speed = i;
				}
				Action::SfxVolume(v) =>
				{
					state.options.sfx_volume = v;
					state.sfx.set_sfx_volume(v);
				}
				Action::UiScale(v) =>
				{
					state.options.ui_scale = v;
				}
				Action::Back =>
				{
					game_state::save_options(&state.core, &state.options).unwrap();
					return Some(Action::Back);
				}
				_ => return Some(action),
			}
		}
		None
	}

	pub fn resize(&mut self, state: &game_state::GameState)
	{
		let cx = state.buffer_width() / 2.;
		let cy = state.buffer_height() / 2.;
		self.widgets.pos.x = cx;
		self.widgets.pos.y = cy;
		self.widgets.resize(state);
	}
}

pub struct InGameMenu
{
	widgets: WidgetList,
}

impl InGameMenu
{
	pub fn new(state: &game_state::GameState) -> Self
	{
		let w = BUTTON_WIDTH;
		let h = BUTTON_HEIGHT;

		let widgets = WidgetList::new(&[
			&[Widget::Button(Button::new(w, h, "Resume", Action::Back))],
			&[Widget::Button(Button::new(
				w,
				h,
				"Controls",
				Action::Forward(|s| Ok(SubScreen::ControlsMenu(ControlsMenu::new(s)))),
			))],
			&[Widget::Button(Button::new(
				w,
				h,
				"Options",
				Action::Forward(|s| Ok(SubScreen::OptionsMenu(OptionsMenu::new(s)))),
			))],
			&[Widget::Button(Button::new(w, h, "Quit", Action::MainMenu))],
		]);
		let mut res = Self { widgets };
		res.resize(state);
		res
	}

	pub fn draw(&self, state: &game_state::GameState)
	{
		self.widgets.draw(state);
	}

	pub fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		self.widgets.input(state, event)
	}

	pub fn resize(&mut self, state: &game_state::GameState)
	{
		let cx = state.buffer_width() / 2.;
		let cy = state.buffer_height() / 2.;
		self.widgets.pos.x = cx;
		self.widgets.pos.y = cy;
		self.widgets.resize(state);
	}
}

pub enum SubScreen
{
	MainMenu(MainMenu),
	ControlsMenu(ControlsMenu),
	OptionsMenu(OptionsMenu),
	InGameMenu(InGameMenu),
}

impl SubScreen
{
	pub fn draw(&self, state: &game_state::GameState)
	{
		match self
		{
			SubScreen::MainMenu(s) => s.draw(state),
			SubScreen::ControlsMenu(s) => s.draw(state),
			SubScreen::OptionsMenu(s) => s.draw(state),
			SubScreen::InGameMenu(s) => s.draw(state),
		}
	}

	pub fn input(&mut self, state: &mut game_state::GameState, event: &Event) -> Option<Action>
	{
		match self
		{
			SubScreen::MainMenu(s) => s.input(state, event),
			SubScreen::ControlsMenu(s) => s.input(state, event),
			SubScreen::OptionsMenu(s) => s.input(state, event),
			SubScreen::InGameMenu(s) => s.input(state, event),
		}
	}

	pub fn resize(&mut self, state: &game_state::GameState)
	{
		match self
		{
			SubScreen::MainMenu(s) => s.resize(state),
			SubScreen::ControlsMenu(s) => s.resize(state),
			SubScreen::OptionsMenu(s) => s.resize(state),
			SubScreen::InGameMenu(s) => s.resize(state),
		}
	}
}

pub struct SubScreens
{
	pub subscreens: Vec<SubScreen>,
	pub action: Option<Action>,
	pub time_to_transition: f64,
}

const TRANSITION_TIME: f64 = 0.25;

impl SubScreens
{
	pub fn new(state: &game_state::GameState) -> Self
	{
		Self {
			subscreens: vec![],
			action: None,
			time_to_transition: state.core.get_time(),
		}
	}

	pub fn reset_transition(&mut self, state: &game_state::GameState)
	{
		self.time_to_transition = state.core.get_time();
	}

	pub fn draw(&self, state: &game_state::GameState)
	{
		let time = state.core.get_time();
		let f = if self.time_to_transition > time
		{
			-1. + (self.time_to_transition - time) / TRANSITION_TIME
		}
		else
		{
			(1. - (time - self.time_to_transition) / TRANSITION_TIME).max(0.)
		};
		let f = f as f32;
		let mut transform = Transform::identity();
		transform.translate(0., state.buffer_height() * f);
		state.core.use_transform(&transform);
		if let Some(subscreen) = self.subscreens.last()
		{
			subscreen.draw(state);
		}
		state.core.use_transform(&Transform::identity());
	}

	pub fn input(
		&mut self, state: &mut game_state::GameState, event: &Event,
	) -> Result<Option<Action>>
	{
		if self.action.is_none()
		{
			self.action = self.subscreens.last_mut().unwrap().input(state, event);
			if self.action.is_some() && self.action != Some(Action::SelectMe)
			{
				self.time_to_transition = state.core.get_time() + TRANSITION_TIME;
			}
		}
		if let (Some(action), true) = (
			self.action.clone(),
			state.core.get_time() > self.time_to_transition,
		)
		{
			self.action = None;
			match action
			{
				Action::Forward(subscreen_fn) =>
				{
					self.subscreens.push(subscreen_fn(state)?);
				}
				Action::Back =>
				{
					self.subscreens.pop().unwrap();
				}
				action @ _ => return Ok(Some(action)),
			}
		}
		Ok(None)
	}

	pub fn resize(&mut self, state: &game_state::GameState)
	{
		for subscreen in &mut self.subscreens
		{
			subscreen.resize(state);
		}
	}

	pub fn pop(&mut self)
	{
		self.subscreens.pop();
	}

	pub fn push(&mut self, screen: SubScreen)
	{
		self.subscreens.push(screen);
	}

	pub fn is_empty(&self) -> bool
	{
		self.subscreens.is_empty()
	}
}
