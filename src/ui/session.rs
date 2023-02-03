use std::io;
use std::ops::Range;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};
use crate::node::{wrap_text, Displayable, Session};
use super::{BufPrint, Rect, Screen, SessionViewConstraints};

pub struct SessionView {
	items: Vec<Item>,
	cursor: usize,
	start: usize,
	heights: Vec<u16>,
}

impl SessionView {
	pub fn new(items: Vec<Item>, len: u16) -> Self {
		let mut sv = SessionView { items, cursor: 0, start: 0, heights: Vec::new() };
		sv.anchor_top(sv.cursor, len);
		sv
	}

	pub fn resize(&mut self, len: u16, tasks_width: usize) {
		for item in self.items.iter_mut() {
			item.name_splits = wrap_text(&item.name, tasks_width);
		}
		self.anchor_top(self.start, len);
	}

	pub fn cursor_up(&mut self) {
		if self.cursor <= self.start {
			if self.cursor > 0 { self.cursor -= 1 };
			self.anchor_top(self.cursor, self.height());
		} else if self.cursor > self.start + self.heights.len() {
			if self.cursor > 0 { self.cursor -= 1 };
			self.anchor_bottom(self.cursor);
		} else {
			self.cursor -= 1;
		}
	}

	pub fn cursor_down(&mut self) {
		if self.cursor + 1 < self.start {
			if self.cursor + 1 < self.items.len() { self.cursor += 1 };
			self.anchor_top(self.cursor, self.height());
		} else if self.cursor + 1 >= self.start + self.heights.len() {
			if self.cursor + 1 < self.items.len() { self.cursor += 1 };
			self.anchor_bottom(self.cursor);
		} else {
			self.cursor += 1;
		}
	}

	fn anchor_top(&mut self, index: usize, len: u16) {
		self.heights.clear();
		let mut h = 0;
		for item in &self.items[index..] {
			if h + item.height() > len { break }
			self.heights.push(h);
			h += item.height();
		}
		self.start = index;
	}

	fn anchor_bottom(&mut self, index: usize) {
		let mut h = self.height();
		self.heights.clear();
		for item in self.items[..=index].iter().rev() {
			if h < item.height() { break }
			h -= item.height();
			self.heights.push(h);
		}
		self.heights.reverse();
		self.start = index + 1 - self.heights.len();
	}

	fn height(&self) -> u16 {
		self.heights.last().map(|l|
			l + self.items.last().unwrap().height()
		).unwrap_or(0)
	}

	fn window(&self) -> Range<usize> {
		self.start..self.start + self.heights.len()
	}
}

pub struct Item {
	pub session: Session,
	pub id: u64,
	pub name: String,
	pub name_splits: Vec<usize>,
}

impl Item {
	pub fn splits(&self) -> impl Iterator<Item = &str> {
		self.name_splits.windows(2).map(|w| &self.name[w[0]..w[1]])
	}

	fn height(&self) -> u16 {
		(self.name_splits.len() - 1) as _
	}
}

impl BufPrint<SessionView, SessionViewConstraints> for Screen {
	fn bufprint(&mut self, view: &SessionView, constr: &SessionViewConstraints) -> io::Result<&mut Self> {
		let mut h = 0;
		for (i, item) in view.items[view.window()].iter().enumerate() {
			if view.start + i == view.cursor {
				let area = Rect {
					x: constr.session.x,
					y: constr.session.y + h,
					w: constr.session.w + constr.tasks.w + 1,
					h: item.height(),
				};
				self.paint(area, Colors::new(Color::Black, Color::White))?;
			}
			h += item.height();
		}

		h = 0;
		for (i, item) in view.items[view.window()].iter().enumerate() {
			if view.start + i == view.cursor {
				self.print_item(item, h, Colors::new(Color::Black, Color::White), constr)?;
			} else {
				self.print_item(item, h, Colors {
					foreground: Some(Color::White),
					background: None,
				}, constr)?;
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
		self.stdout
			.queue(SetColors(colors))?
			.queue(MoveTo(constr.session.x, constr.session.y + dy))?
			.queue(Print(Displayable(Some(item.session))))?;

		for (i, split) in item.splits().enumerate() {
			self.stdout
				.queue(MoveTo(constr.tasks.x, constr.tasks.y + dy + i as u16))?
				.queue(Print(split))?;
		}
		self.stdout.queue(ResetColor)?;
		Ok(())
	}
}
