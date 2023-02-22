use std::cmp::min;
use std::io;
use std::fmt::{self, Display, Formatter};
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};
use super::{BufPrint, Screen, StatusViewConstraints};

pub struct StatusView {
	input: Input,
	start: usize,
	buffer: String,
	title: &'static str,
	pub constr: StatusViewConstraints,
}

impl StatusView {
	pub fn new() -> io::Result<Self> {
		Ok(StatusView {
			input: Input { front: "".into(), back: "".into() },
			start: 0,
			buffer: "".into(),
			title: "",
			constr: StatusViewConstraints::new()?,
		})
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
		if self.input.front.len() - self.start >= self.cmd_width() {
			self.start = self.input.front.len() - usize::from(self.cmd_width()) + 1;
		}
	}

	pub fn delete(&mut self) {
		self.input.front.pop();
		if self.input.front.len() < self.start + self.cmd_width() / 2 {
			self.start = self.input.front.len().saturating_sub(self.cmd_width() / 2);
		}
	}

	pub fn move_left(&mut self) {
		if let Some(c) = self.input.front.pop() {
			self.input.back.push(c);
			if self.input.front.len() < self.start + self.cmd_width() / 2 {
				self.start = self.input.front.len().saturating_sub(self.cmd_width() / 2);
			}
		}
	}

	pub fn move_right(&mut self) {
		if let Some(c) = self.input.back.pop() {
			self.input.front.push(c);
			if self.input.front.len() - self.start >= self.cmd_width() / 2 {
				self.start = self.input.front.len() - usize::from(self.cmd_width() / 2) + 1;
			}
		}
	}

	pub fn clear(&mut self) {
		self.start = 0;
		self.input.front.clear();
		self.input.back.clear();
	}

	pub fn input(&mut self) -> &str {
		self.buffer.clear();
		self.buffer += &self.input.front;
		self.input.back.chars().rev().for_each(|c| self.buffer.push(c));
		&self.buffer
	}

	fn cmd_width(&self) -> usize {
		usize::from(self.constr.status.w) - self.title.len()
	}
}

struct Input {
	front: String,
	back: String,
}

impl BufPrint<StatusView> for Screen {
	fn bufprint(&mut self, view: &StatusView) -> io::Result<&mut Self> {
		self.stdout
			.queue(MoveTo(view.constr.status.x, view.constr.status.y))?
			.queue(Print(view.title))?
			.queue(Print(&view.input.front[view.start..]))?
			.queue(SetColors(Colors::new(Color::Black, Color::White)))?
			.queue(Print(view.input.back.chars().rev().next().unwrap_or(' ')))?
			.queue(Print(ResetColor))?;

		if !view.input.back.is_empty() {
			let till = min(view.input.back.len(), view.cmd_width() + view.start - view.input.front.len() - 1);
			self.stdout.queue(Print(Reverse(&view.input.back[view.input.back.len() - till..view.input.back.len() - 1])))?;
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
