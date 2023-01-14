use std::io;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Print, PrintStyledContent, Stylize};
use super::{BufPrint, Screen};

pub struct StatusView {
	pub command: String,
	title: &'static str,
}

impl StatusView {
	pub fn new() -> Self {
		StatusView {
			command: String::new(),
			title: "",
		}
	}

	pub fn set_title(&mut self, title: &'static str) {
		self.title = title;
	}
}

impl BufPrint<StatusView> for Screen {
	fn bufprint(&mut self, view: &StatusView) -> io::Result<&mut Self> {
		self.stdout
			.queue(MoveTo(self.constr.status.x, self.constr.status.y))?
			.queue(Print(view.title))?
			.queue(Print(&view.command))?
			.queue(PrintStyledContent(" ".on(Color::White)))?;

		Ok(self)
	}
}
