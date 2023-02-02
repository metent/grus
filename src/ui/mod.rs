pub mod session;
pub mod status;
pub mod tree;

use std::io::{self, stdout, Stdout, Write};
use crossterm::{terminal, QueueableCommand, ExecutableCommand};
use crossterm::cursor::MoveTo;
use crossterm::style::{Colors, Print, ResetColor, SetColors};
use crossterm::terminal::{Clear, ClearType};

pub struct Screen {
	stdout: Stdout,
}

impl Screen {
	pub fn new() -> io::Result<Self> {
		Ok(Screen { stdout: stdout() })
	}

	pub fn paint(&mut self, area: Rect, colors: Colors) -> io::Result<()> {
		self.stdout.queue(SetColors(colors))?;
		for y in area.y..area.y + area.h {
			for x in area.x..area.x + area.w {
				self.stdout.queue(MoveTo(x, y))?.queue(Print(' '))?;
			}
		}
		self.stdout.queue(ResetColor)?;
		Ok(())
	}

	pub fn clear(&mut self) -> io::Result<&mut Self> {
		self.stdout.execute(Clear(ClearType::All))?;
		Ok(self)
	}

	pub fn flush(&mut self) -> io::Result<()> {
		self.stdout.flush()
	}
}

pub trait BufPrint<V, C> {
	fn bufprint(&mut self, view: &V, constr: &C) -> io::Result<&mut Self>;
}

#[derive(Default)]
pub struct TreeViewConstraints {
	w: u16,
	h: u16,
	tasks: Rect,
	session: Rect,
	due_date: Rect,
}

impl TreeViewConstraints {
	pub fn new() -> io::Result<Self> {
		let mut constr = TreeViewConstraints::default();
		let (w, h) = terminal::size()?;
		constr.update(w, h);
		Ok(constr)
	}

	pub fn update(&mut self, w: u16, h: u16) {
		if h < 2 { return }
		self.w = w;
		self.h = h;
		self.tasks = Rect { x: 1, y: 1, w: (w - 1) / 2, h: h - 2 };
		self.session = Rect {
			x: self.tasks.x + self.tasks.w + 1,
			y: 1,
			w: (w - 1) / 3,
			h: h - 2
		};
		self.due_date = Rect {
			x: self.session.x + self.session.w + 1,
			y: 1,
			w: w.saturating_sub(4 + self.tasks.w + self.session.w),
			h: h - 2
		};
	}

	pub fn height(&self) -> u16 {
		self.h
	}

	pub fn tree_width(&self) -> usize {
		self.tasks.w.into()
	}

	pub fn tree_height(&self) -> usize {
		self.tasks.h.into()
	}
}

#[derive(Default)]
pub struct SessionViewConstraints {
	session: Rect,
	tasks: Rect,
}

impl SessionViewConstraints {
	pub fn new() -> io::Result<Self> {
		let mut constr = SessionViewConstraints::default();
		let (w, h) = terminal::size()?;
		constr.update(w, h);
		Ok(constr)
	}

	pub fn update(&mut self, w: u16, h: u16) {
		if h < 2 { return };
		self.session = Rect { x: 1, y: 1, w: (w - 1) / 2, h: h - 2 };
		self.tasks = Rect {
			x: self.session.x + self.session.w + 1,
			y: 1,
			w: (w - 1) / 2,
			h: h - 2,
		};
	}

	pub fn session_height(&self) -> usize {
		self.session.h.into()
	}
}

#[derive(Default)]
pub struct StatusViewConstraints {
	status: Rect,
}

impl StatusViewConstraints {
	pub fn new() -> io::Result<Self> {
		let mut constr = StatusViewConstraints::default();
		let (w, h) = terminal::size()?;
		constr.update(w, h);
		Ok(constr)
	}

	pub fn update(&mut self, w: u16, h: u16) {
		self.status = Rect { x: 0, y: h - 1, w, h: 1 };
	}
}

#[derive(Default)]
pub struct Rect {
	x: u16,
	y: u16,
	w: u16,
	pub h: u16,
}
