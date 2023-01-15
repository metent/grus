use std::io;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};
use crate::node::{Node, Displayable};
use super::{BufPrint, Screen};

pub struct TreeView {
	flattree: Vec<Node<'static>>,
	sel: usize,
}

impl TreeView {
	pub fn new(flattree: Vec<Node<'static>>) -> Self {
		TreeView { flattree, sel: 0 }
	}

	pub fn reset(&mut self, flattree: Vec<Node<'static>>, ret: SelRetention) {
		match ret {
			SelRetention::Stay => (),
			SelRetention::MoveUp => self.sel -= 1,
			SelRetention::SameId => if let Some(&Node { id, pid, .. }) = self.sel_node() {
				if let Some(sel) = flattree.iter().position(|node| node.id == id) {
					self.sel = sel;
				} else if let Some(sel) = flattree.iter().position(|node| node.pid == pid) {
					self.sel = sel;
				} else {
					self.sel = 0
				}
			}
			SelRetention::Reset => self.sel = 0,
		}
		self.flattree = flattree;
	}

	pub fn move_sel_up(&mut self) {
		if self.sel > 0 {
			self.sel -= 1;
		}
	}

	pub fn move_sel_down(&mut self) {
		if self.sel < self.flattree.len() - 1 {
			self.sel += 1;
		}
	}

	pub fn sel_node(&self) -> Option<&Node> {
		if self.flattree.len() > 0 {
			Some(&self.flattree[self.sel])
		} else {
			None
		}
	}

	pub fn is_root_selected(&self) -> bool {
		self.sel == 0
	}

}

pub enum SelRetention {
	Stay,
	MoveUp,
	SameId,
	Reset,
}

impl BufPrint<TreeView> for Screen {
	fn bufprint(&mut self, view: &TreeView) -> io::Result<&mut Self> {
		let Some(sel_node) = view.sel_node() else { return Ok(self) };
		let sel_h = view.flattree.iter().enumerate().take_while(|&(i, _)| i < view.sel)
			.fold(0, |acc, (_, task)| acc + task.height());
		self.paint(
			self.constr.tasks.x,
			self.constr.tasks.y + sel_h as u16,
			self.constr.tasks.w + self.constr.priority.w + self.constr.due_date.w + 2,
			sel_node.splits.len() as u16 - 1
		)?;

		let mut h = 0;
		for (i, task) in view.flattree.iter().enumerate() {
			if i == view.sel {
				self.print_sel_task(task, h)?;
			} else {
				self.print_task(task, h)?;
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
	fn print_sel_task(&mut self, task: &Node, dy: u16) -> io::Result<()>;
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

	fn print_sel_task(&mut self, task: &Node, dy: u16) -> io::Result<()> {
		self.stdout
			.queue(SetColors(Colors::new(Color::Black, Color::White)))?
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
