use std::cmp::min;
use std::io;
use std::ops::Range;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};
use crate::node::{Displayable, Session};
use super::{BufPrint, Screen, SessionViewConstraints};

pub struct SessionView {
	items: Vec<Item>,
	cursor: usize,
	window: Range<usize>,
}

impl SessionView {
	pub fn new(items: Vec<Item>, len: usize) -> Self {
		let total = items.len();
		SessionView { items, cursor: 0, window: 0..min(len, total) }
	}

	pub fn resize(&mut self, len: usize) {
		self.window.end = min(self.window.start + len, self.items.len());
	}

	pub fn cursor_up(&mut self) {
		if self.cursor <= self.window.start {
			if self.cursor > 0 { self.cursor -= 1 };
			self.window.end = self.cursor + self.window.end - self.window.start;
			self.window.start = self.cursor;
		} else if self.cursor > self.window.end {
			if self.cursor > 0 { self.cursor -= 1 };
			self.window.start = self.cursor + self.window.start + 1 - self.window.end;
			self.window.end = self.cursor + 1;
		} else {
			self.cursor -= 1;
		}
	}

	pub fn cursor_down(&mut self) {
		if self.cursor + 1 < self.window.start {
			if self.cursor + 1 < self.items.len() { self.cursor += 1 };
			self.window.end = self.cursor + self.window.end - self.window.start;
			self.window.start = self.cursor;
		} else if self.cursor + 1 >= self.window.end {
			if self.cursor + 1 < self.items.len() { self.cursor += 1 };
			self.window.start = self.cursor + self.window.start + 1 - self.window.end;
			self.window.end = self.cursor + 1;
		} else {
			self.cursor += 1;
		}
	}

	fn window(&self) -> Range<usize> {
		self.window.clone()
	}
}

pub struct Item {
	pub session: Session,
	pub id: u64,
	pub name: String,
}

impl BufPrint<SessionView, SessionViewConstraints> for Screen {
	fn bufprint(&mut self, view: &SessionView, constr: &SessionViewConstraints) -> io::Result<&mut Self> {
		let mut h = 0;
		for (i, item) in view.items[view.window()].iter().enumerate() {
			if view.window.start + i == view.cursor {
				self.print_item(item, h, Colors::new(Color::Black, Color::White), constr)?;
			} else {
				self.print_item(item, h, Colors {
					foreground: Some(Color::White),
					background: None,
				}, constr)?;
			}
			h += 1;
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
			.queue(Print(Displayable(Some(item.session))))?
			.queue(MoveTo(constr.tasks.x, constr.tasks.y + dy))?
			.queue(Print(&item.name))?
			.queue(ResetColor)?;
		Ok(())
	}
}
