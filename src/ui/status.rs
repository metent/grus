use std::cmp::min;
use std::io;
use std::fmt::{self, Display, Formatter};
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};
use super::{BufPrint, Screen, StatusViewConstraints};

pub struct StatusView<const V: usize> {
	input: Input,
	start: usize,
	buffer: String,
	pub mode: Mode,
	pub constr: StatusViewConstraints,
}

impl<const V: usize> StatusView<V> {
	pub fn new() -> io::Result<Self> {
		Ok(StatusView {
			input: Input { front: "".into(), back: "".into() },
			start: 0,
			buffer: "".into(),
			mode: Mode::Normal,
			constr: StatusViewConstraints::new()?,
		})
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
		if let Mode::Command(cmd_type) = self.mode {
			usize::from(self.constr.status.w) - COMMAND_TEXT[cmd_type as usize].len() - VIEW_TEXT[V].len()
		} else { 0 }
	}
}

#[derive(Copy, Clone)]
pub enum Mode {
	Normal,
	Command(CommandType),
}

#[derive(Copy, Clone)]
pub enum CommandType {
	AddChild,
	Rename,
	SetDueDate,
	AddSession,
}

impl Display for CommandType {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", COMMAND_TEXT[*self as usize])
	}
}

const VIEW_TEXT: &[&str] = &[
	" TREE VIEW ",
	" SESSION VIEW ",
];

const COMMAND_TEXT: &[&str] = &[
	"add: ",
	"rename: ",
	"due date: ",
	"add session: ",
];

struct Input {
	front: String,
	back: String,
}

impl<const V: usize> BufPrint<StatusView<V>> for Screen {
	fn bufprint(&mut self, view: &StatusView<V>) -> io::Result<&mut Self> {
		self.stdout
			.queue(MoveTo(view.constr.status.x + view.constr.status.w - VIEW_TEXT[V].len() as u16, view.constr.status.y))?
			.queue(Print(VIEW_TEXT[V]))?;

		let Mode::Command(cmd_type) = view.mode else { return Ok(self) };
		self.stdout
			.queue(MoveTo(view.constr.status.x, view.constr.status.y))?
			.queue(Print(Print(cmd_type)))?
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
