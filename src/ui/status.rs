use std::io;
use std::fmt::{self, Display, Formatter};
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};
use super::{BufPrint, Screen, StatusViewConstraints};

pub struct StatusView {
	input: Input,
	buffer: String,
	title: &'static str,
}

impl StatusView {
	pub fn new() -> Self {
		StatusView {
			input: Input { front: "".into(), back: "".into() },
			buffer: "".into(),
			title: "",
		}
	}

	pub fn set_title(&mut self, title: &'static str) {
		self.title = title;
	}

	pub fn set_input(&mut self, input: &str) {
		self.input.front.clear();
		self.input.front += input;
		self.input.back.clear();
	}

	pub fn insert(&mut self, c: char) {
		self.input.front.push(c);
	}

	pub fn delete(&mut self) {
		self.input.front.pop();
	}

	pub fn move_left(&mut self) {
		if let Some(c) = self.input.front.pop() {
			self.input.back.push(c);
		}
	}

	pub fn move_right(&mut self) {
		if let Some(c) = self.input.back.pop() {
			self.input.front.push(c);
		}
	}

	pub fn clear(&mut self) {
		self.input.front.clear();
		self.input.back.clear();
	}

	pub fn input(&mut self) -> &str {
		self.buffer.clear();
		self.buffer += &self.input.front;
		self.input.back.chars().rev().for_each(|c| self.buffer.push(c));
		&self.buffer
	}
}

struct Input {
	front: String,
	back: String,
}

impl BufPrint<StatusView, StatusViewConstraints> for Screen {
	fn bufprint(&mut self, view: &StatusView, constr: &StatusViewConstraints) -> io::Result<&mut Self> {
		self.stdout
			.queue(MoveTo(constr.status.x, constr.status.y))?
			.queue(Print(view.title))?
			.queue(Print(&view.input.front))?
			.queue(SetColors(Colors::new(Color::Black, Color::White)))?
			.queue(Print(view.input.back.chars().rev().next().unwrap_or(' ')))?
			.queue(Print(ResetColor))?;

		if !view.input.back.is_empty() {
			self.stdout.queue(Print(Reverse(&view.input.back[..view.input.back.len() - 1])))?;
		}

		Ok(self)
	}
}

struct Reverse<'a>(&'a str);

impl<'a> Display for Reverse<'a> {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		for c in self.0.chars().rev() {
			write!(f, "{}", c)?;
		}
		Ok(())
	}
}
