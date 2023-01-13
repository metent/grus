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
			.fold(0, |acc, (_, node)| acc + node.splits.len() - 1);
		for i in 0..sel_node.splits.len() - 1 {
			for x in self.constr.tasks.x..self.constr.tasks.x + self.constr.tasks.w + self.constr.priority.w + self.constr.due_date.w + 2 {
				self.stdout
					.queue(MoveTo(x, self.constr.tasks.y + sel_h as u16 + i as u16))?
					.queue(SetColors(Colors::new(Color::Black, Color::White)))?
					.queue(Print(' '))?
					.queue(ResetColor)?;
			}
		}

		let mut h = 0;
		for (i, task) in view.flattree.iter().enumerate() {
			if i == view.sel {
				self.stdout
					.queue(SetColors(Colors::new(Color::Black, Color::White)))?
					.queue(MoveTo(
						self.constr.priority.x,
						self.constr.priority.y + h
					))?
					.queue(Print(task.data.priority))?
					.queue(MoveTo(
						self.constr.due_date.x,
						self.constr.due_date.y + h
					))?
					.queue(Print(Displayable(task.data.due_date)))?;

				for split in task.splits.windows(2).map(|w| &task.data.name[w[0]..w[1]]) {
					self.stdout
						.queue(MoveTo(
							self.constr.tasks.x + 2 * task.depth as u16,
							self.constr.tasks.y + h
						))?
						.queue(Print(split))?;
					h += 1;
				}

				self.stdout.queue(ResetColor)?;
			} else {
				self.stdout
					.queue(MoveTo(
						self.constr.priority.x,
						self.constr.priority.y + h
					))?
					.queue(Print(task.data.priority))?
					.queue(MoveTo(
						self.constr.due_date.x,
						self.constr.due_date.y + h
					))?
					.queue(Print(Displayable(task.data.due_date)))?;

				for split in task.splits.windows(2).map(|w| &task.data.name[w[0]..w[1]]) {
					self.stdout
						.queue(MoveTo(
							self.constr.tasks.x + 2 * task.depth as u16,
							self.constr.tasks.y + h
						))?
						.queue(Print(split))?;
					h += 1;
				}
			}
		}

		return Ok(self);
	}
}
