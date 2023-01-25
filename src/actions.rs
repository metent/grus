use std::collections::HashSet;
use std::str::FromStr;
use chrono::Local;
use interim::{parse_date_string, Dialect};
use crate::app::{Application, CommandType, Error, Mode};
use crate::flattree::{FlatTreeBuilder, FlatTreeState};
use crate::node::{Node, NodeData, Priority, Session, wrap_text};
use crate::store::{Store, StoreReader};
use crate::ui::tree::SelRetention;

pub trait Actions {
	fn enter_command_mode(&mut self, cmd: CommandType);
	fn add_child(&mut self) -> Result<(), Error>;
	fn delete(&mut self) -> Result<(), Error>;
	fn rename(&mut self) -> Result<(), Error>;
	fn set_due_date(&mut self) -> Result<(), Error>;
	fn unset_due_date(&mut self) -> Result<(), Error>;
	fn add_session(&mut self) -> Result<(), Error>;
	fn priority_up(&mut self) -> Result<(), Error>;
	fn priority_down(&mut self) -> Result<(), Error>;
	fn share(&mut self) -> Result<(), Error>;
	fn cut(&mut self) -> Result<(), Error>;
	fn move_into(&mut self) -> Result<(), Error>;
	fn move_out(&mut self) -> Result<(), Error>;
	fn cancel(&mut self);
	fn resize(&mut self, width: u16, height: u16) -> Result<(), Error>;
}

impl Actions for Application {
	fn enter_command_mode(&mut self, cmd: CommandType) {
		let Some(node) = self.tree_view.cursor_node() else { return };

		self.mode = Mode::Command(cmd);
		let title = match cmd {
			CommandType::AddChild => "add: ",
			CommandType::Rename => {
				self.status_view.set_input(&node.data.name);
				"rename: "
			}
			CommandType::SetDueDate => "due date: ",
			CommandType::AddSession => "add session: ",
		};
		self.status_view.set_title(title);
	}

	fn add_child(&mut self) -> Result<(), Error> {
		let mut selections = self.tree_view.selection_ids();
		let Some(&first) = selections.next() else { return Ok(()) };

		let name = self.status_view.input();
		let data = bincode::serialize(&NodeData::with_name(name))?;

		let mut writer = self.store.writer()?;
		let id = writer.add_child(first, &data)?;

		for &pid in selections {
			writer.share(id, pid)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(SelRetention::SameId)?;

		self.cancel();
		Ok(())
	}

	fn delete(&mut self) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };
		if self.tree_view.is_cursor_at_root() { return Ok(()) }

		let mut writer = self.store.writer()?;
		writer.delete(node.pid, node.id)?;
		writer.commit()?;

		self.tree_view.deselect();
		self.update_tree_view(SelRetention::Parent)?;
		Ok(())
	}

	fn rename(&mut self) -> Result<(), Error> {
		let mut writer = self.store.writer()?;
		for &id in self.tree_view.selection_ids() {
			let original = bincode::deserialize(writer.read(id)?.unwrap())?;

			let name = self.status_view.input();
			let data = bincode::serialize(&NodeData { name: name.into(), ..original })?;

			writer.modify(id, &data)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(SelRetention::SameId)?;

		self.cancel();
		Ok(())
	}

	fn set_due_date(&mut self) -> Result<(), Error> {
		let mut writer = self.store.writer()?;
		for &id in self.tree_view.selection_ids() {
			let original = bincode::deserialize(writer.read(id)?.unwrap())?;

			let Ok(due_date) = parse_date_string(self.status_view.input(), Local::now(), Dialect::Uk) else {
				return Ok(());
			};
			let data = bincode::serialize(&NodeData { due_date: Some(due_date.naive_local()), ..original })?;

			writer.modify(id, &data)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(SelRetention::SameId)?;

		self.cancel();
		Ok(())
	}

	fn unset_due_date(&mut self) -> Result<(), Error> {
		let mut writer = self.store.writer()?;
		for &id in self.tree_view.selection_ids() {
			let original = bincode::deserialize(writer.read(id)?.unwrap())?;

			let data = bincode::serialize(&NodeData { due_date: None, ..original })?;

			writer.modify(id, &data)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(SelRetention::SameId)?;
		Ok(())
	}

	fn add_session(&mut self) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		let mut writer = self.store.writer()?;

		let Ok(session) = Session::from_str(&self.status_view.input()) else { return Ok(()) };
		writer.add_session(node.id, &session)?;
		writer.commit()?;

		self.update_tree_view(SelRetention::SameId)?;
		self.cancel();
		Ok(())
	}

	fn priority_up(&mut self) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		if self.tree_view.is_cursor_at_root() { return Ok(()) };

		let mut writer = self.store.writer()?;
		writer.move_up(node.pid, node.id)?;
		writer.commit()?;

		self.update_tree_view(SelRetention::SameId)?;
		Ok(())
	}

	fn priority_down(&mut self) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		if self.tree_view.is_cursor_at_root() { return Ok(()) };

		let mut writer = self.store.writer()?;
		writer.move_down(node.pid, node.id)?;
		writer.commit()?;

		self.update_tree_view(SelRetention::SameId)?;
		Ok(())
	}

	fn share(&mut self) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		let mut writer = self.store.writer()?;
		for &id in self.tree_view.selection_ids() {
			if !writer.share(id, node.id)? { return Ok(()) };
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(SelRetention::Stay)?;
		Ok(())
	}

	fn cut(&mut self) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		let mut writer = self.store.writer()?;
		for (&pid, &id) in self.tree_view.selections() {
			if !writer.cut(pid, id, node.id)? { return Ok(()) };
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(SelRetention::Stay)?;
		Ok(())
	}

	fn move_into(&mut self) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		if self.tree_view.is_cursor_at_root() { return Ok(()) }

		self.stack.push(self.root_id);
		self.root_id = node.id;

		self.update_tree_view(SelRetention::Reset)?;
		Ok(())
	}

	fn move_out(&mut self) -> Result<(), Error> {
		let Some(root_id) = self.stack.pop() else { return Ok(()) };
		self.root_id = root_id;

		self.update_tree_view(SelRetention::Reset)?;
		Ok(())
	}

	fn cancel(&mut self) {
		self.status_view.clear();
		self.mode = Mode::Normal;
	}

	fn resize(&mut self, width: u16, height: u16) -> Result<(), Error> {
		self.screen.update(width, height);
		self.update_tree_view(SelRetention::SameId)?;
		Ok(())
	}
}

trait Update {
	fn update_tree_view(&mut self, ret: SelRetention) -> Result<(), Error>;
}

impl Update for Application {
	fn update_tree_view(&mut self, ret: SelRetention) -> Result<(), Error> {
		let height = self.screen.tree_height().into();
		let width = self.screen.tree_width().into();
		let flattree = build_flattree(self.root_id, &self.store, height, width)?;
		self.tree_view.reset(flattree, ret);
		Ok(())
	}
}

pub fn build_flattree(
	id: u64,
	store: &Store,
	height: usize,
	width: usize,
) -> Result<Vec<Node<'static>>, Error> {
	if width <= 1 { return Ok(Vec::new()) }

	let reader = store.reader()?;
	let root_data: NodeData = bincode::deserialize(reader.read(id)?.unwrap())?;
	let root_splits = wrap_text(&root_data.name, width.saturating_sub(1));
	if root_splits.len() - 1 > height { return Ok(Vec::new()) }
	let root = Node { id, pid: id, data: root_data, session: reader.read_session(id)?.map(|s| *s), priority: Priority::default(), depth: 0, splits: root_splits };
	let mut builder = FlatTreeBuilder::new(root, height);
	let mut ids = HashSet::new();

	loop {
		match builder.step() {
			FlatTreeState::Build => continue,
			FlatTreeState::Refill => {
				for i in builder.fill_range() {
					let id = builder.id(i);
					if ids.insert(id) {
						builder.fill(get_children(id, &reader, builder.depth(i), width)?, i);
					}
				}
				builder.finish_fill();
			}
			FlatTreeState::Done => return Ok(builder.finish()),
		}
	}
}

fn get_children(
	pid: u64,
	reader: &StoreReader,
	mut depth: usize,
	width: usize
) -> Result<Vec<Node<'static>>, Error> {
	depth += 1;
	let mut children = Vec::new();
	for entry in reader.children(pid)? {
		let (id, data) = entry?;
		let data: NodeData = bincode::deserialize(data)?;
		let width = width.saturating_sub(2 * depth + 1);
		if width == 0 { continue }
		let splits = wrap_text(&data.name, width);
		children.push(Node {
			id,
			pid,
			depth,
			data,
			session: reader.read_session(id)?.map(|s| *s),
			priority: Priority::default(),
			splits
		});
	}
	for i in 0..children.len() {
		children[i].priority = Priority {
			det: i as u64,
			total: children.len() as u64,
		};
	}
	Ok(children)
}
