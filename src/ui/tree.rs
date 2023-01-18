use std::collections::{HashMap, HashSet};
use std::io;
use std::iter;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};
use crate::node::{Node, Displayable};
use super::{BufPrint, Rect, Screen};

pub struct TreeView {
	flattree: Vec<Node<'static>>,
	cursor: usize,
	selections: HashMap<u64, HashSet<u64>>,
}

impl TreeView {
	pub fn new(flattree: Vec<Node<'static>>) -> Self {
		TreeView { flattree, cursor: 0, selections: HashMap::new() }
	}

	pub fn reset(&mut self, flattree: Vec<Node<'static>>, ret: SelRetention) {
		match ret {
			SelRetention::Stay => (),
			SelRetention::Parent => if let Some(&Node { pid, .. }) = self.cursor_node() {
				if let Some(cursor) = flattree.iter().position(|node| node.id == pid) {
					self.cursor = cursor;
				} else {
					self.cursor = 0
				}
			}
			SelRetention::SameId => if let Some(&Node { id, pid, .. }) = self.cursor_node() {
				if let Some(cursor) = flattree.iter().position(|node| node.id == id) {
					self.cursor = cursor;
				} else if let Some(cursor) = flattree.iter().position(|node| node.id == pid) {
					self.cursor = cursor;
				} else {
					self.cursor = 0
				}
			}
			SelRetention::Reset => self.cursor = 0,
		}
		self.flattree = flattree;
	}

	pub fn toggle(&mut self) {
		let Some(&Node { id, pid, .. }) = self.cursor_node() else { return };

		if let Some(pids) = self.selections.get_mut(&id) {
			if !pids.insert(pid) {
				pids.remove(&pid);
				if pids.is_empty() {
					self.selections.remove(&id);
				}
			}
		} else {
			self.selections.insert(id, HashSet::from([pid]));
		}
	}

	pub fn deselect(&mut self) {
		let Some(&Node { id, pid, .. }) = self.cursor_node() else { return };

		if let Some(pids) = self.selections.get_mut(&id) {
			pids.remove(&pid);
			if pids.is_empty() {
				self.selections.remove(&id);
			}
		}
	}

	pub fn clear_selections(&mut self) {
		self.selections.clear();
	}

	pub fn cursor_up(&mut self) {
		if self.cursor > 0 {
			self.cursor -= 1;
		}
	}

	pub fn cursor_down(&mut self) {
		if self.cursor < self.flattree.len() - 1 {
			self.cursor += 1;
		}
	}

	pub fn cursor_node(&self) -> Option<&Node> {
		if self.flattree.len() > 0 {
			Some(&self.flattree[self.cursor])
		} else {
			None
		}
	}

	pub fn selections(&self) -> impl Iterator<Item = (&u64, &u64)> {
		if !self.selections.is_empty() {
			Selections::Actual(self.selections.iter().flat_map(|(id, pids)| pids.iter().zip(iter::repeat(id))))
		} else if let Some(Node { id, pid, .. }) = self.cursor_node() {
			Selections::Cursor(iter::once((pid, id)))
		} else {
			Selections::Empty(iter::empty())
		}
	}

	pub fn selection_ids(&self) -> impl Iterator<Item = &u64> {
		if !self.selections.is_empty() {
			SelectionIds::Actual(self.selections.keys())
		} else if let Some(Node { id, .. }) = self.cursor_node() {
			SelectionIds::Cursor(iter::once(id))
		} else {
			SelectionIds::Empty(iter::empty())
		}
	}

	pub fn is_selected(&self, pid: u64, id: u64) -> bool {
		if let Some(pids) = self.selections.get(&id) {
			pids.contains(&pid)
		} else {
			false
		}
	}

	pub fn is_cursor_at_root(&self) -> bool {
		self.cursor == 0
	}
}

pub enum Selections<'s, T: Iterator<Item = (&'s u64, &'s u64)>> {
	Cursor(iter::Once<(&'s u64, &'s u64)>),
	Actual(T),
	Empty(iter::Empty<(&'s u64, &'s u64)>),
}

impl<'s, T: Iterator<Item = (&'s u64, &'s u64)>> Iterator for Selections<'s, T> {
	type Item = (&'s u64, &'s u64);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Selections::Cursor(iter) => iter.next(),
			Selections::Actual(iter) => iter.next(),
			Selections::Empty(iter) => iter.next(),
		}
	}
}

pub enum SelectionIds<'s, T: Iterator<Item = &'s u64>> {
	Cursor(iter::Once<&'s u64>),
	Actual(T),
	Empty(iter::Empty<&'s u64>),
}

impl<'s, T: Iterator<Item = &'s u64>> Iterator for SelectionIds<'s, T> {
	type Item = &'s u64;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			SelectionIds::Cursor(iter) => iter.next(),
			SelectionIds::Actual(iter) => iter.next(),
			SelectionIds::Empty(iter) => iter.next(),
		}
	}
}

pub enum SelRetention {
	Stay,
	Parent,
	SameId,
	Reset,
}

impl BufPrint<TreeView> for Screen {
	fn bufprint(&mut self, view: &TreeView) -> io::Result<&mut Self> {
		let mut h = 0;
		for (i, task) in view.flattree.iter().enumerate() {
			let area = Rect {
				x: self.constr.tasks.x,
				y: self.constr.tasks.y + h as u16,
				w: self.constr.tasks.w + self.constr.priority.w + self.constr.due_date.w + 2,
				h: task.height(),
			};

			match (i == view.cursor, view.is_selected(task.pid, task.id)) {
				(true, true) => self.paint(area, Colors::new(Color::White, Color::Blue))?,
				(true, false) => self.paint(area, Colors::new(Color::Black, Color::White))?,
				(false, true) => self.paint(area, Colors::new(Color::White, Color::DarkBlue))?,
				(false, false) => {},
			}

			h += task.height();
		}

		h = 0;
		for (i, task) in view.flattree.iter().enumerate() {
			match (i == view.cursor, view.is_selected(task.pid, task.id)) {
				(true, true) =>
					self.print_sel_task(task, h, Colors::new(Color::White, Color::Blue))?,
				(true, false) =>
					self.print_sel_task(task, h, Colors::new(Color::Black, Color::White))?,
				(false, true) =>
					self.print_sel_task(task, h, Colors::new(Color::White, Color::DarkBlue))?,
				(false, false) =>
					self.print_task(task, h)?,
			}
			h += task.height();
		}

		let mut line_pos = Vec::new();
		for task in view.flattree.iter().rev() {
			h -= task.height();
			match line_pos.last() {
				Some(&last) if task.depth < last => {
					line_pos.pop();
					if line_pos.last() == Some(&task.depth) {
						self.print_div_lines(task, h, &line_pos, false)?;
					} else {
						line_pos.push(task.depth);
						self.print_div_lines(task, h, &line_pos, true)?;
					}
				}
				Some(&last) if task.depth == last => {
					self.print_div_lines(task, h, &line_pos, false)?;
				}
				_ => {
					line_pos.push(task.depth);
					self.print_div_lines(task, h, &line_pos, true)?;
				}
			}
		}

		return Ok(self);
	}
}

trait PrintTask {
	fn print_task(&mut self, task: &Node, dy: u16) -> io::Result<()>;
	fn print_sel_task(&mut self, task: &Node, dy: u16, colors: Colors) -> io::Result<()>;
	fn print_div_lines(
		&mut self,
		task: &Node,
		dy: u16,
		line_pos: &[usize],
		is_last: bool
	) -> io::Result<()>;
}

impl PrintTask for Screen {
	fn print_task(&mut self, task: &Node, dy: u16) -> io::Result<()> {
		self.stdout
			.queue(MoveTo(self.constr.priority.x, self.constr.priority.y + dy))?
			.queue(Print(task.data.priority))?
			.queue(MoveTo(self.constr.due_date.x, self.constr.due_date.y + dy))?
			.queue(Print(Displayable(task.data.due_date)))?;

		for (i, split) in task.splits().enumerate() {
			self.stdout
				.queue(MoveTo(
					self.constr.tasks.x + 2 * task.depth as u16,
					self.constr.tasks.y + dy + i as u16,
				))?
				.queue(Print(split))?;
		}
		Ok(())
	}

	fn print_sel_task(&mut self, task: &Node, dy: u16, colors: Colors) -> io::Result<()> {
		self.stdout
			.queue(SetColors(colors))?
			.queue(MoveTo(self.constr.priority.x, self.constr.priority.y + dy))?
			.queue(Print(task.data.priority))?
			.queue(MoveTo(self.constr.due_date.x, self.constr.due_date.y + dy))?
			.queue(Print(Displayable(task.data.due_date)))?;

		for (i, split) in task.splits().enumerate() {
			self.stdout
				.queue(MoveTo(
					self.constr.tasks.x + 2 * task.depth as u16,
					self.constr.tasks.y + dy + i as u16
				))?
				.queue(Print(split))?;
		}

		self.stdout.queue(ResetColor)?;
		Ok(())
	}

	fn print_div_lines(
		&mut self,
		task: &Node,
		dy: u16,
		line_pos: &[usize],
		is_last: bool
	) -> io::Result<()> {
		if task.depth == 0 { return Ok(()) };
		for dy in dy..dy + task.height() {
			self.stdout.queue(MoveTo(self.constr.tasks.x, self.constr.tasks.y + dy))?;
			let mut pos_iter = line_pos.iter();
			let mut pos = pos_iter.next();
			for d in 1..task.depth {
				if Some(&d) == pos {
					self.stdout.queue(Print("│ "))?;
					pos = pos_iter.next();
				} else {
					self.stdout.queue(Print("  "))?;
				}
			}
			if is_last {
				self.stdout.queue(Print("  "))?;
			} else {
				self.stdout.queue(Print("│ "))?;
			}
		}

		let dx = 2 * task.depth as u16 - 2;
		self.stdout.queue(MoveTo(self.constr.tasks.x + dx, self.constr.tasks.y + dy))?;
		if is_last {
			self.stdout.queue(Print("└─"))?;
		} else {
			self.stdout.queue(Print("├─"))?;
		}
		Ok(())
	}
}
