pub mod status;
pub mod tree;

use std::io::{self, stdout, Stdout, Write};
use crossterm::{terminal, ExecutableCommand};
use crossterm::terminal::{Clear, ClearType};

pub struct Screen {
	stdout: Stdout,
	constr: Constraints,
}

impl Screen {
	pub fn init() -> io::Result<Self> {
		let mut screen = Screen { stdout: stdout(), constr: Constraints::default() };

		let (w, h) = terminal::size()?;
		screen.update(w, h);

		Ok(screen)
	}

	pub fn update(&mut self, w: u16, h: u16) {
		if h < 2 { return }
		self.constr.w = w;
		self.constr.h = h;
		self.constr.tasks = Rect { x: 1, y: 1, w: 2 * (w - 1) / 3, h: h - 2 };
		self.constr.priority = Rect {
			x: self.constr.tasks.x + self.constr.tasks.w + 1,
			y: 1,
			w: (w - 1) / 6,
			h: h - 2
		};
		self.constr.due_date = Rect {
			x: self.constr.priority.x + self.constr.priority.w + 1,
			y: 1,
			w: w.saturating_sub(4 + self.constr.tasks.w + self.constr.priority.w),
			h: h - 2
		};
		self.constr.status = Rect { x: 0, y: h - 1, w, h: 1 };
	}

	pub fn height(&self) -> u16 {
		self.constr.h
	}

	pub fn tree_width(&self) -> u16 {
		self.constr.tasks.w
	}

	pub fn tree_height(&self) -> u16 {
		self.constr.tasks.h
	}

	pub fn clear(&mut self) -> io::Result<&mut Self> {
		self.stdout.execute(Clear(ClearType::All))?;
		Ok(self)
	}

	pub fn flush(&mut self) -> io::Result<()> {
		self.stdout.flush()
	}
}

pub trait BufPrint<T> {
	fn bufprint(&mut self, view: &T) -> io::Result<&mut Self>;
}

#[derive(Default)]
pub struct Constraints {
	w: u16,
	h: u16,
	tasks: Rect,
	priority: Rect,
	due_date: Rect,
	status: Rect,
}

#[derive(Default)]
pub struct Rect {
	x: u16,
	y: u16,
	w: u16,
	pub h: u16,
}
