use crate::app::{Application, CommandType, Error, Mode};
use crate::flattree::{FlatTreeBuilder, FlatTreeState};
use crate::node::{Node, NodeData, Priority, wrap_text};
use crate::store::{Store, StoreReader};
use crate::ui::tree::SelRetention;

pub trait Actions {
	fn enter_command_mode(&mut self, cmd: CommandType);
	fn add_child(&mut self) -> Result<(), Error>;
	fn delete(&mut self) -> Result<(), Error>;
	fn rename(&mut self) -> Result<(), Error>;
	fn set_priority(&mut self, priority: Priority) -> Result<(), Error>;
	fn set_due_date(&mut self) -> Result<(), Error>;
	fn unset_due_date(&mut self) -> Result<(), Error>;
	fn move_into(&mut self) -> Result<(), Error>;
	fn move_out(&mut self) -> Result<(), Error>;
	fn cancel(&mut self);
	fn resize(&mut self, width: u16, height: u16) -> Result<(), Error>;
}

impl Actions for Application {
	fn enter_command_mode(&mut self, cmd: CommandType) {
		let Some(sel_node) = self.tree_view.sel_node() else { return };

		self.mode = Mode::Command(cmd);
		let title = match cmd {
			CommandType::AddChild => "add: ",
			CommandType::Rename => {
				self.status_view.command.push_str(&sel_node.data.name);
				"rename: "
			}
			CommandType::SetDueDate => "due date: ",
		};
		self.status_view.set_title(title);
	}

	fn add_child(&mut self) -> Result<(), Error> {
		let Some(sel_node) = self.tree_view.sel_node() else { return Ok(()) };

		let name = &self.status_view.command;
		let data = bincode::serialize(&NodeData::with_name(name))?;

		let mut writer = self.store.writer()?;
		writer.add_child(sel_node.id, &data)?;
		writer.commit()?;

		self.update_tree_view(SelRetention::Stay)?;

		self.cancel();
		Ok(())
	}

	fn delete(&mut self) -> Result<(), Error> {
		let Some(sel_node) = self.tree_view.sel_node() else { return Ok(()) };
		if self.tree_view.is_root_selected() { return Ok(()) }

		let mut writer = self.store.writer()?;
		writer.delete(sel_node.pid, sel_node.id)?;
		writer.commit()?;

		self.update_tree_view(SelRetention::MoveUp)?;
		Ok(())
	}

	fn rename(&mut self) -> Result<(), Error> {
		let Some(sel_node) = self.tree_view.sel_node() else { return Ok(()) };

		let reader = self.store.reader()?;
		let original = bincode::deserialize(reader.read(sel_node.id)?.unwrap())?;
		drop(reader);

		let name = &self.status_view.command;
		let data = bincode::serialize(&NodeData { name: name.into(), ..original })?;

		let mut writer = self.store.writer()?;
		writer.modify(sel_node.id, &data)?;
		writer.commit()?;

		self.update_tree_view(SelRetention::SameId)?;

		self.cancel();
		Ok(())
	}

	fn set_priority(&mut self, priority: Priority) -> Result<(), Error> {
		let Some(sel_node) = self.tree_view.sel_node() else { return Ok(()) };

		let reader = self.store.reader()?;
		let original = bincode::deserialize(reader.read(sel_node.id)?.unwrap())?;
		drop(reader);

		let data = bincode::serialize(&NodeData { priority, ..original })?;

		let mut writer = self.store.writer()?;
		writer.modify(sel_node.id, &data)?;
		writer.commit()?;

		self.update_tree_view(SelRetention::SameId)?;

		self.cancel();
		Ok(())
	}

	fn set_due_date(&mut self) -> Result<(), Error> {
		let Some(sel_node) = self.tree_view.sel_node() else { return Ok(()) };

		let reader = self.store.reader()?;
		let original = bincode::deserialize(reader.read(sel_node.id)?.unwrap())?;
		drop(reader);

		let Ok(due_date) = fuzzydate::parse(&self.status_view.command) else {
			return Ok(());
		};
		let data = bincode::serialize(&NodeData { due_date: Some(due_date), ..original })?;

		let mut writer = self.store.writer()?;
		writer.modify(sel_node.id, &data)?;
		writer.commit()?;

		self.update_tree_view(SelRetention::SameId)?;

		self.cancel();
		Ok(())
	}

	fn unset_due_date(&mut self) -> Result<(), Error> {
		let Some(sel_node) = self.tree_view.sel_node() else { return Ok(()) };

		let reader = self.store.reader()?;
		let original = bincode::deserialize(reader.read(sel_node.id)?.unwrap())?;
		drop(reader);

		let data = bincode::serialize(&NodeData { due_date: None, ..original })?;

		let mut writer = self.store.writer()?;
		writer.modify(sel_node.id, &data)?;
		writer.commit()?;

		self.update_tree_view(SelRetention::SameId)?;
		Ok(())
	}

	fn move_into(&mut self) -> Result<(), Error> {
		let Some(sel_node) = self.tree_view.sel_node() else { return Ok(()) };

		if self.tree_view.is_root_selected() { return Ok(()) }

		self.stack.push(self.root_id);
		self.root_id = sel_node.id;

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
		self.status_view.command.clear();
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
	if width == 0 { return Ok(Vec::new()) }

	let reader = store.reader()?;
	let root_data: NodeData = bincode::deserialize(reader.read(id)?.unwrap())?;
	let root_splits = wrap_text(&root_data.name, width);
	if root_splits.len() - 1 > height { return Ok(Vec::new()) }
	let root = Node { id, pid: id, data: root_data, depth: 0, splits: root_splits };
	let mut builder = FlatTreeBuilder::new(root, height);

	loop {
		match builder.step() {
			FlatTreeState::Build => continue,
			FlatTreeState::Refill => {
				for i in builder.fill_range() {
					builder.fill(get_children(builder.id(i), &reader, builder.depth(i), width)?, i);
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
		let width = width.saturating_sub(2 * depth);
		if width == 0 { continue }
		let splits = wrap_text(&data.name, width);
		children.push(Node { id, pid, depth, data, splits });
	}
	Ok(children)
}
