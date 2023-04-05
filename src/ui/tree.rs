use std::collections::{HashMap, HashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io;
use std::iter;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors, SetForegroundColor};
use crate::node::{Node, Priority};
use super::{BufPrint, Rect, Screen, TreeViewConstraints};

pub struct TreeView {
	flattree: Vec<Node<'static>>,
	cursor: usize,
	selections: HashMap<u64, HashSet<u64>>,
	pub root_id: u64,
	pub stack: Vec<u64>,
	pub constr: TreeViewConstraints,
}

impl TreeView {
	pub fn new(flattree: Vec<Node<'static>>) -> io::Result<Self> {
		Ok(TreeView {
			flattree,
			cursor: 0,
			selections: HashMap::new(),
			root_id: 0,
			stack: Vec::new(),
			constr: TreeViewConstraints::new()?,
		})
	}

	pub fn reset(&mut self, flattree: Vec<Node<'static>>) {
		let Some(&Node { id, pid, priority, .. }) = self.cursor_node() else {
			self.cursor = 0;
			self.flattree = flattree;
			return
		};
		let mut same = None;
		let mut next = None;
		let mut prev = None;
		let mut parent = None;
		for (i, node) in flattree.iter().enumerate() {
			if node.id == id && node.pid == pid { same = Some(i); break }
			else if node.pid == pid && node.priority.det == priority.det { next = Some(i) }
			else if node.pid == pid && node.priority.det + 1 == priority.det { prev = Some(i) }
			else if parent.is_none() && node.id == pid { parent = Some(i) }
		}
		self.cursor = same.or(next).or(prev).or(parent).unwrap_or(0);

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
		if self.cursor + 1 < self.flattree.len() {
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

impl BufPrint<TreeView> for Screen {
	fn bufprint(&mut self, view: &TreeView) -> io::Result<&mut Self> {
		let mut painter = TreeViewPainter::new(self, view, &view.constr)?;
		painter.paint_sel_task()?;
		painter.print_tasks()?;
		painter.paint_div_lines()?;
		painter.paint_col_lines()?;
		Ok(self)
	}
}

struct TreeViewPainter<'screen, 'view, 'constr> {
	screen: &'screen mut Screen,
	view: &'view TreeView,
	constr: &'constr TreeViewConstraints,
	height: u16,
	color_map: HashMap<u64, Color>,
}

impl<'screen, 'view, 'constr> TreeViewPainter<'screen, 'view, 'constr> {
	fn new(screen: &'screen mut Screen, view: &'view TreeView, constr: &'constr TreeViewConstraints) -> io::Result<Self> {
		let mut h = 0;
		let mut color_map = HashMap::new();
		for task in view.flattree.iter() {
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

			h += task.height() as u16;
		}
		Ok(TreeViewPainter { screen, view, constr, color_map, height: h })
	}

	fn print_tasks(&mut self) -> io::Result<()> {
		let mut h = 0;
		for (i, task) in self.view.flattree.iter().enumerate() {
			match (i == self.view.cursor, self.view.is_selected(task.pid, task.id)) {
				(true, true) =>
					self.print_task(task, h, Colors::new(Color::White, Color::Blue))?,
				(true, false) =>
					self.print_task(task, h, Colors::new(Color::Black, Color::White))?,
				(false, true) =>
					self.print_task(task, h, Colors::new(Color::White, Color::DarkBlue))?,
				(false, false) => self.print_task(task, h, Colors {
					foreground: Some(Color::White),
					background: None,
				})?,
			}
			h += task.height() as u16;
		}
		Ok(())
	}

	fn print_task(&mut self, task: &Node, dy: u16, colors: Colors) -> io::Result<()> {
		self.screen.stdout.queue(SetColors(colors))?;

		for (i, split) in task.name_splits().enumerate() {
			self.screen.stdout
				.queue(MoveTo(
					self.constr.tasks.x + 2 * task.depth as u16 + 1,
					self.constr.tasks.y + dy + i as u16
				))?
				.queue(Print(split))?;
		}

		for (i, split) in task.session_splits().enumerate() {
			self.screen.stdout
				.queue(MoveTo(
					self.constr.session.x,
					self.constr.session.y + dy + i as u16
				))?
				.queue(Print(split))?;
		}

		for (i, split) in task.due_date_splits().enumerate() {
			self.screen.stdout
				.queue(MoveTo(
					self.constr.due_date.x,
					self.constr.due_date.y + dy + i as u16
				))?
				.queue(Print(split))?;
		}

		self.screen.stdout.queue(ResetColor)?;
		Ok(())
	}

	fn paint_sel_task(&mut self) -> io::Result<()> {
		let mut h = 0;
		for (i, task) in self.view.flattree.iter().enumerate() {
			let area = Rect {
				x: self.constr.tasks.x,
				y: self.constr.tasks.y + h as u16,
				w: self.constr.tasks.w + self.constr.session.w + self.constr.due_date.w + 2,
				h: task.height() as u16,
			};

			match (i == self.view.cursor, self.view.is_selected(task.pid, task.id)) {
				(true, true) => self.screen.paint(area, Colors::new(Color::White, Color::Blue))?,
				(true, false) => self.screen.paint(area, Colors::new(Color::Black, Color::White))?,
				(false, true) => self.screen.paint(area, Colors::new(Color::White, Color::DarkBlue))?,
				(false, false) => {},
			}

			h += task.height() as u16;
		}
		Ok(())
	}

	fn paint_div_lines(&mut self) -> io::Result<()> {
		let mut line_pos = Vec::new();
		let mut h = self.height;
		let mut prev_depth = 0;
		for task in self.view.flattree.iter().rev() {
			h -= task.height() as u16;

			let is_next_child = prev_depth == task.depth + 1;
			prev_depth = task.depth;

			match line_pos.last() {
				Some(&last) if task.depth < last => {
					line_pos.pop();
					if line_pos.last() == Some(&task.depth) {
						self.paint_div_line(task, h, &line_pos, is_next_child)?;
					} else {
						line_pos.push(task.depth);
						self.paint_div_line(task, h, &line_pos, is_next_child)?;
					}
				}
				Some(&last) if task.depth == last => {
					self.paint_div_line(task, h, &line_pos, is_next_child)?;
				}
				_ => {
					line_pos.push(task.depth);
					self.paint_div_line(task, h, &line_pos, is_next_child)?;
				}
			}
		}

		Ok(())
	}

	fn paint_div_line(
		&mut self,
		task: &Node,
		dy: u16,
		line_pos: &[usize],
		next_is_child: bool,
	) -> io::Result<()> {
		if task.depth == 0 {
			if next_is_child {
				for dy in dy..dy + task.height() as u16 {
					self.screen.stdout.queue(MoveTo(self.constr.tasks.x, self.constr.tasks.y + dy))?;
					self.screen.stdout.queue(Print("│"))?;
				}
			}
			self.screen.stdout
				.queue(MoveTo(self.constr.tasks.x, self.constr.tasks.y))?
				.queue(SetForegroundColor(color_from_prio(&task.priority)))?
				.queue(Print("•"))?
				.queue(ResetColor)?;
			return Ok(());
		}

		let color = *self.color_map.get(&task.id).unwrap();
		for dy in dy..dy + task.height() as u16 {
			self.screen.stdout.queue(MoveTo(self.constr.tasks.x, self.constr.tasks.y + dy))?;
			let mut pos_iter = line_pos.iter();
			let mut pos = pos_iter.next();
			for d in 1..task.depth {
				if Some(&d) == pos {
					self.screen.stdout.queue(Print("│ "))?;
					pos = pos_iter.next();
				} else {
					self.screen.stdout.queue(Print("  "))?;
				}
			}
			self.screen.stdout
				.queue(Print(if task.priority.is_least() { "  " } else { "│ " }))?
				.queue(Print(if next_is_child { "│" } else { " " }))?;
		}

		let dx = 2 * task.depth as u16 - 2;
		self.screen.stdout
			.queue(MoveTo(self.constr.tasks.x + dx, self.constr.tasks.y + dy))?;
		if color != Color::White {
			self.screen.stdout
				.queue(SetForegroundColor(color))?
				.queue(Print(if task.priority.is_least() { "┕━" } else { "┝━" }))?
				.queue(ResetColor)?;
		} else {
			self.screen.stdout.queue(Print(if task.priority.is_least() { "└─" } else { "├─" }))?;
		}
		self.screen.stdout
			.queue(SetForegroundColor(color_from_prio(&task.priority)))?
			.queue(Print("•"))?
			.queue(ResetColor)?;
		Ok(())
	}

	fn paint_col_lines(&mut self) -> io::Result<()> {
		let mut h = 0;
		for (i, task) in self.view.flattree.iter().enumerate() {
			match (i == self.view.cursor, self.view.is_selected(task.pid, task.id)) {
				(true, true) => self.screen.stdout.queue(SetColors(Colors::new(Color::White, Color::Blue)))?,
				(true, false) => self.screen.stdout.queue(SetColors(Colors::new(Color::Black, Color::White)))?,
				(false, true) => self.screen.stdout.queue(SetColors(Colors::new(Color::White, Color::DarkBlue)))?,
				(false, false) => &mut self.screen.stdout,
			};

			self.screen.draw_vline(self.constr.session.x - 1, self.constr.session.y + h, task.height() as u16)?;
			self.screen.draw_vline(self.constr.due_date.x - 1, self.constr.due_date.y + h, task.height() as u16)?;
			self.screen.stdout.queue(ResetColor)?;

			h += task.height() as u16;
		}
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
