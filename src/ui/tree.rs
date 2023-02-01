use std::collections::{HashMap, HashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io;
use std::iter;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors, SetForegroundColor};
use crate::node::{Node, Displayable, Priority};
use super::{BufPrint, Rect, Screen, TreeViewConstraints};

pub struct TreeView {
	flattree: Vec<Node<'static>>,
	cursor: usize,
	selections: HashMap<u64, HashSet<u64>>,
	pub root_id: u64,
	pub stack: Vec<u64>,
}

impl TreeView {
	pub fn new(flattree: Vec<Node<'static>>) -> Self {
		TreeView { flattree, cursor: 0, selections: HashMap::new(), root_id: 0, stack: Vec::new() }
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

	pub fn move_into(&mut self) {
		let Some(&Node { id, .. }) = self.cursor_node() else { return };
		if self.is_cursor_at_root() { return };

		self.stack.push(self.root_id);
		self.root_id = id;
	}

	pub fn move_out(&mut self) {
		let Some(root_id) = self.stack.pop() else { return };
		self.root_id = root_id;
	}

	pub fn root_id(&self) -> u64 {
		self.root_id
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

impl BufPrint<TreeView, TreeViewConstraints> for Screen {
	fn bufprint(&mut self, view: &TreeView, constr: &TreeViewConstraints) -> io::Result<&mut Self> {
		let mut h = 0;
		for (i, task) in view.flattree.iter().enumerate() {
			let area = Rect {
				x: constr.tasks.x,
				y: constr.tasks.y + h as u16,
				w: constr.tasks.w + constr.session.w + constr.due_date.w + 2,
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
		let mut color_map = HashMap::new();
		for (i, task) in view.flattree.iter().enumerate() {
			if let Some(color) = color_map.get_mut(&task.id) {
				if let Color::White = color {
					let mut hasher = DefaultHasher::new();
					hasher.write_u64(task.id);
					let hash = hasher.finish().to_le_bytes()[0];
					*color = Color::AnsiValue(hash);
				}
			} else {
				color_map.insert(task.id, Color::White);
			}

			match (i == view.cursor, view.is_selected(task.pid, task.id)) {
				(true, true) =>
					self.print_task(task, h, Colors::new(Color::White, Color::Blue), constr)?,
				(true, false) =>
					self.print_task(task, h, Colors::new(Color::Black, Color::White), constr)?,
				(false, true) =>
					self.print_task(task, h, Colors::new(Color::White, Color::DarkBlue), constr)?,
				(false, false) => self.print_task(task, h, Colors {
					foreground: Some(Color::White),
					background: None,
				}, constr)?,
			}
			h += task.height();
		}

		let mut line_pos = Vec::new();
		for task in view.flattree.iter().rev() {
			h -= task.height();

			let color = *color_map.get(&task.id).unwrap();
			match line_pos.last() {
				Some(&last) if task.depth < last => {
					line_pos.pop();
					if line_pos.last() == Some(&task.depth) {
						self.print_div_lines(task, h, &line_pos, false, color, constr)?;
					} else {
						line_pos.push(task.depth);
						self.print_div_lines(task, h, &line_pos, true, color, constr)?;
					}
				}
				Some(&last) if task.depth == last => {
					self.print_div_lines(task, h, &line_pos, false, color, constr)?;
				}
				_ => {
					line_pos.push(task.depth);
					self.print_div_lines(task, h, &line_pos, true, color, constr)?;
				}
			}
		}

		return Ok(self);
	}
}

trait PrintTask {
	fn print_task(&mut self, task: &Node, dy: u16, colors: Colors, constr: &TreeViewConstraints) -> io::Result<()>;
	fn print_div_lines(
		&mut self,
		task: &Node,
		dy: u16,
		line_pos: &[usize],
		is_last: bool,
		color: Color,
		constr: &TreeViewConstraints,
	) -> io::Result<()>;
}

impl PrintTask for Screen {
	fn print_task(&mut self, task: &Node, dy: u16, colors: Colors, constr: &TreeViewConstraints) -> io::Result<()> {
		self.stdout
			.queue(SetColors(colors))?
			.queue(MoveTo(constr.session.x, constr.session.y + dy))?
			.queue(Print(Displayable(task.session)))?
			.queue(MoveTo(constr.due_date.x, constr.due_date.y + dy))?
			.queue(Print(Displayable(task.data.due_date)))?;

		for (i, split) in task.splits().enumerate() {
			self.stdout
				.queue(MoveTo(
					constr.tasks.x + 2 * task.depth as u16 + 1,
					constr.tasks.y + dy + i as u16
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
		is_last: bool,
		color: Color,
		constr: &TreeViewConstraints,
	) -> io::Result<()> {
		if task.depth == 0 {
			self.stdout
				.queue(MoveTo(constr.tasks.x, constr.tasks.y))?
				.queue(SetForegroundColor(color_from_prio(&task.priority)))?
				.queue(Print("•"))?
				.queue(ResetColor)?;
			return Ok(());
		}

		for dy in dy..dy + task.height() {
			self.stdout.queue(MoveTo(constr.tasks.x, constr.tasks.y + dy))?;
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
				self.stdout.queue(Print("   "))?;
			} else {
				self.stdout.queue(Print("│  "))?;
			}
		}

		let dx = 2 * task.depth as u16 - 2;
		self.stdout
			.queue(MoveTo(constr.tasks.x + dx, constr.tasks.y + dy))?;
		if color != Color::White {
			self.stdout.queue(SetForegroundColor(color))?;
			if is_last {
				self.stdout.queue(Print("┕━"))?;
			} else {
				self.stdout.queue(Print("┝━"))?;
			}
			self.stdout.queue(ResetColor)?;
		} else {
			if is_last {
				self.stdout.queue(Print("└─"))?;
			} else {
				self.stdout.queue(Print("├─"))?;
			}
		}
		self.stdout
			.queue(SetForegroundColor(color_from_prio(&task.priority)))?
			.queue(Print("•"))?
			.queue(ResetColor)?;
		Ok(())
	}
}

fn color_from_prio(prio: &Priority) -> Color {
	color_from_hsv((prio.det * 120) as f64 / prio.total as f64, 1.0, 1.0)
}

fn color_from_hsv(hue: f64, saturation: f64, value: f64) -> Color {
	let c = value * saturation;
	let h = hue / 60.0;
	let x = c * (1.0 - (h % 2.0 - 1.0).abs());
	let m = value - c;

	let (red, green, blue) = if h >= 0.0 && h < 1.0 {
		(c, x, 0.0)
	} else if h >= 1.0 && h < 2.0 {
		(x, c, 0.0)
	} else if h >= 2.0 && h < 3.0 {
		(0.0, c, x)
	} else if h >= 3.0 && h < 4.0 {
		(0.0, x, c)
	} else if h >= 4.0 && h < 5.0 {
		(x, 0.0, c)
	} else {
		(c, 0.0, x)
	};

	Color::Rgb {
		r: ((red + m) * 255.0) as u8,
		g: ((green + m) * 255.0) as u8,
		b: ((blue + m) * 255.0) as u8,
	}
}
