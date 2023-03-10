use std::cmp::max;
use std::io;
use std::ops::Range;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};
use grus_lib::types::Session;
use crate::node::wrap_text;
use super::{BufPrint, Rect, Screen, SessionViewConstraints};

pub struct SessionView {
	items: Vec<Item>,
	cursor: usize,
	start: usize,
	heights: Vec<u16>,
	pub constr: SessionViewConstraints,
}

impl SessionView {
	pub fn new(items: Vec<Item>) -> io::Result<Self> {
		let mut sv = SessionView {
			items,
			cursor: 0,
			start: 0,
			heights: Vec::new(),
			constr: SessionViewConstraints::new()?,
		};
		sv.anchor_top(sv.cursor);
		Ok(sv)
	}

	pub fn reset(&mut self, items: Vec<Item>) {
		if self.items.len() == 0 {
			self.items = items;
			self.anchor_top(self.start);
			return;
		}
		let pos = match items.binary_search(&self.items[self.cursor]) {
			Ok(pos) => pos,
			Err(pos) => pos,
		};
		if pos < items.len() {
			self.cursor = pos;
		} else {
			self.cursor = items.len().saturating_sub(1);
		}

		let start = match items.binary_search(&self.items[self.start]) {
			Ok(start) => start,
			Err(start) => start,
		};
		if start < items.len() {
			self.start = start;
		} else {
			self.start = items.len().saturating_sub(1);
		}
		self.items = items;
		self.anchor_top(self.start);
	}

	pub fn resize(&mut self, tasks_width: usize, session_width: usize) {
		for item in self.items.iter_mut() {
			item.name_splits = wrap_text(&item.name, tasks_width);
			item.session_splits = wrap_text(&item.session_text, session_width);
		}
		self.anchor_top(self.start);
	}

	pub fn cursor_up(&mut self) {
		if self.cursor <= self.start {
			if self.cursor > 0 { self.cursor -= 1 };
			self.anchor_top(self.cursor);
		} else if self.cursor > self.start + self.heights.len() {
			if self.cursor > 0 { self.cursor -= 1 };
			self.anchor_bottom(self.cursor);
		} else {
			self.cursor -= 1;
		}
	}

	pub fn cursor_down(&mut self) {
		if self.items.is_empty() { return }
		if self.cursor + 1 < self.start {
			if self.cursor + 1 < self.items.len() { self.cursor += 1 };
			self.anchor_top(self.cursor);
		} else if self.cursor + 1 >= self.start + self.heights.len() {
			if self.cursor + 1 < self.items.len() { self.cursor += 1 };
			self.anchor_bottom(self.cursor);
		} else {
			self.cursor += 1;
		}
	}

	pub fn session_and_id(&self) -> Option<(u64, &Session)> {
		self.items.get(self.cursor).map(|Item { id, session, .. }| (*id, session))
	}

	fn anchor_top(&mut self, index: usize) {
		self.heights.clear();
		let mut h = 0;
		for item in &self.items[index..] {
			if h + item.height() > self.constr.session_height() { break }
			self.heights.push(h);
			h += item.height();
		}
		self.start = index;
	}

	fn anchor_bottom(&mut self, index: usize) {
		let mut h = self.constr.session_height();
		self.heights.clear();
		for item in self.items[..=index].iter().rev() {
			if h < item.height() { break }
			h -= item.height();
			self.heights.push(h);
		}
		self.heights.reverse();
		self.start = index + 1 - self.heights.len();
	}

	fn window(&self) -> Range<usize> {
		self.start..self.start + self.heights.len()
	}
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct Item {
	pub session: Session,
	pub id: u64,
	pub name: String,
	pub name_splits: Vec<usize>,
	pub session_text: String,
	pub session_splits: Vec<usize>,
}

impl Item {
	pub fn name_splits(&self) -> impl Iterator<Item = &str> {
		self.name_splits.windows(2).map(|w| &self.name[w[0]..w[1]])
	}

	pub fn session_splits(&self) -> impl Iterator<Item = &str> {
		self.session_splits.windows(2).map(|w| &self.session_text[w[0]..w[1]])
	}

	fn height(&self) -> u16 {
		max(self.session_splits.len() - 1, self.name_splits.len() - 1) as _
	}
}

impl BufPrint<SessionView> for Screen {
	fn bufprint(&mut self, view: &SessionView) -> io::Result<&mut Self> {
		let mut h = 0;
		for (i, item) in view.items[view.window()].iter().enumerate() {
			if view.start + i == view.cursor {
				let area = Rect {
					x: view.constr.session.x,
					y: view.constr.session.y + h,
					w: view.constr.session.w + view.constr.tasks.w + 1,
					h: item.height(),
				};
				self.paint(area, Colors::new(Color::Black, Color::White))?;
			}
			h += item.height();
		}

		h = 0;
		for (i, item) in view.items[view.window()].iter().enumerate() {
			if view.start + i == view.cursor {
				self.print_item(item, h, Colors::new(Color::Black, Color::White), &view.constr)?;
			} else {
				self.print_item(item, h, Colors {
					foreground: Some(Color::White),
					background: None,
				}, &view.constr)?;
			}
			h += item.height();
		}
		Ok(self)
	}
}

trait PrintItem {
	fn print_item(&mut self, item: &Item, dy: u16, colors: Colors, constr: &SessionViewConstraints) -> io::Result<()>;
}

impl PrintItem for Screen {
	fn print_item(&mut self, item: &Item, dy: u16, colors: Colors, constr: &SessionViewConstraints) -> io::Result<()> {
		self.stdout.queue(SetColors(colors))?;

		for (i, split) in item.session_splits().enumerate() {
			self.stdout
				.queue(MoveTo(constr.session.x, constr.session.y + dy + i as u16))?
				.queue(Print(split))?;
		}

		for (i, split) in item.name_splits().enumerate() {
			self.stdout
				.queue(MoveTo(constr.tasks.x, constr.tasks.y + dy + i as u16))?
				.queue(Print(split))?;
		}
		self.stdout.queue(ResetColor)?;
		Ok(())
	}
}
