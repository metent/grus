use std::collections::HashSet;
use std::io;
use std::str::FromStr;
use chrono::Local;
use crossterm::event::{self, KeyCode, Event};
use interim::{parse_date_string, Dialect};
use crate::app::{Action, CommandType, Error, Mode, View};
use crate::flattree::{FlatTreeBuilder, FlatTreeState};
use crate::node::{Node, NodeData, Priority, Session, wrap_text};
use crate::store::{Store, StoreReader};
use crate::ui::{BufPrint, Screen, StatusViewConstraints, TreeViewConstraints};
use crate::ui::tree::{SelRetention, TreeView};
use crate::ui::status::StatusView;

pub struct TreeViewController {
	tree_view: TreeView,
	tvconstr: TreeViewConstraints,
	status_view: StatusView,
	svconstr: StatusViewConstraints,
	mode: Mode,
}

impl TreeViewController {
	pub fn new(store: &Store) -> Result<Self, Error> {
		let tvconstr = TreeViewConstraints::new()?;
		let flattree = build_flattree(0, &store, tvconstr.tree_width(), tvconstr.tree_height())?;
		Ok(TreeViewController {
			tree_view: TreeView::new(flattree),
			tvconstr,
			status_view: StatusView::new(),
			svconstr: StatusViewConstraints::new()?,
			mode: Mode::Normal,
		})
	}

	pub fn run(&mut self, store: &Store) -> Result<Action, Error> {
		match event::read()? {
			Event::Key(kev) => match self.mode {
				Mode::Normal => match kev.code {
					KeyCode::Char('q') => return Ok(Action::Quit),
					KeyCode::Char('j') | KeyCode::Down => self.tree_view.cursor_down(),
					KeyCode::Char('k') | KeyCode::Up => self.tree_view.cursor_up(),
					KeyCode::Char('h') | KeyCode::Left => self.move_out(store)?,
					KeyCode::Char('l') | KeyCode::Right => self.move_into(store)?,
					KeyCode::Char(' ') => self.tree_view.toggle(),
					KeyCode::Char('.') => self.share(store)?,
					KeyCode::Char('x') => self.cut(store)?,
					KeyCode::Char('a') => self.enter_command_mode(CommandType::AddChild),
					KeyCode::Char('r') => self.enter_command_mode(CommandType::Rename),
					KeyCode::Char('z') => self.enter_command_mode(CommandType::SetDueDate),
					KeyCode::Char('s') => self.enter_command_mode(CommandType::AddSession),
					KeyCode::Char('Z') => self.unset_due_date(store)?,
					KeyCode::Char('K') => self.priority_up(store)?,
					KeyCode::Char('J') => self.priority_down(store)?,
					KeyCode::Char('d') => self.delete(store)?,
					KeyCode::Char('2') => return Ok(Action::Switch(View::Session)),
					_ => {},
				}
				Mode::Command(cmd) => match kev.code {
					KeyCode::Enter => {
						match cmd {
							CommandType::AddChild => self.add_child(store)?,
							CommandType::Rename => self.rename(store)?,
							CommandType::SetDueDate => self.set_due_date(store)?,
							CommandType::AddSession => self.add_session(store)?,
						}
					}
					KeyCode::Char(c) => self.status_view.insert(c),
					KeyCode::Backspace => _ = self.status_view.delete(),
					KeyCode::Left => self.status_view.move_left(),
					KeyCode::Right => self.status_view.move_right(),
					KeyCode::Esc => self.cancel(),
					_ => {},
				}
			}
			Event::Resize(w, h) => self.resize(store, w, h)?,
			_ => {},
		}
		Ok(Action::None)
	}

	pub fn resize(&mut self, store: &Store, w: u16, h: u16) -> Result<(), Error> {
		self.tvconstr.update(w, h);
		self.svconstr.update(w, h);
		self.update_tree_view(store, SelRetention::SameId)?;
		Ok(())
	}

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

	fn add_child(&mut self, store: &Store) -> Result<(), Error> {
		let mut selections = self.tree_view.selection_ids();
		let Some(&first) = selections.next() else { return Ok(()) };

		let name = self.status_view.input();
		let data = bincode::serialize(&NodeData::with_name(name))?;

		let mut writer = store.writer()?;
		let id = writer.add_child(first, &data)?;

		for &pid in selections {
			writer.share(id, pid)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(store, SelRetention::SameId)?;

		self.cancel();
		Ok(())
	}

	fn delete(&mut self, store: &Store) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };
		if self.tree_view.is_cursor_at_root() { return Ok(()) }

		let mut writer = store.writer()?;
		writer.delete(node.pid, node.id)?;
		writer.commit()?;

		self.tree_view.deselect();
		self.update_tree_view(store, SelRetention::Parent)?;
		Ok(())
	}

	fn rename(&mut self, store: &Store) -> Result<(), Error> {
		let mut writer = store.writer()?;
		for &id in self.tree_view.selection_ids() {
			let original = bincode::deserialize(writer.read(id)?.unwrap())?;

			let name = self.status_view.input();
			let data = bincode::serialize(&NodeData { name: name.into(), ..original })?;

			writer.modify(id, &data)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(store, SelRetention::SameId)?;

		self.cancel();
		Ok(())
	}

	fn set_due_date(&mut self, store: &Store) -> Result<(), Error> {
		let mut writer = store.writer()?;
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
		self.update_tree_view(store, SelRetention::SameId)?;

		self.cancel();
		Ok(())
	}

	fn unset_due_date(&mut self, store: &Store) -> Result<(), Error> {
		let mut writer = store.writer()?;
		for &id in self.tree_view.selection_ids() {
			let original = bincode::deserialize(writer.read(id)?.unwrap())?;

			let data = bincode::serialize(&NodeData { due_date: None, ..original })?;

			writer.modify(id, &data)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(store, SelRetention::SameId)?;
		Ok(())
	}

	fn add_session(&mut self, store: &Store) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		let mut writer = store.writer()?;

		let Ok(session) = Session::from_str(&self.status_view.input()) else { return Ok(()) };
		writer.add_session(node.id, &session)?;
		writer.commit()?;

		self.update_tree_view(store, SelRetention::SameId)?;
		self.cancel();
		Ok(())
	}

	fn priority_up(&mut self, store: &Store) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		if self.tree_view.is_cursor_at_root() { return Ok(()) };

		let mut writer = store.writer()?;
		writer.move_up(node.pid, node.id)?;
		writer.commit()?;

		self.update_tree_view(store, SelRetention::SameId)?;
		Ok(())
	}

	fn priority_down(&mut self, store: &Store) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		if self.tree_view.is_cursor_at_root() { return Ok(()) };

		let mut writer = store.writer()?;
		writer.move_down(node.pid, node.id)?;
		writer.commit()?;

		self.update_tree_view(store, SelRetention::SameId)?;
		Ok(())
	}

	fn share(&mut self, store: &Store) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		let mut writer = store.writer()?;
		for &id in self.tree_view.selection_ids() {
			if !writer.share(id, node.id)? { return Ok(()) };
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(store, SelRetention::Stay)?;
		Ok(())
	}

	fn cut(&mut self, store: &Store) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		let mut writer = store.writer()?;
		for (&pid, &id) in self.tree_view.selections() {
			if !writer.cut(pid, id, node.id)? { return Ok(()) };
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(store, SelRetention::Stay)?;
		Ok(())
	}

	fn move_into(&mut self, store: &Store) -> Result<(), Error> {
		self.tree_view.move_into();
		self.update_tree_view(store, SelRetention::Reset)?;
		Ok(())
	}

	fn move_out(&mut self, store: &Store) -> Result<(), Error> {
		self.tree_view.move_out();
		self.update_tree_view(store, SelRetention::Reset)?;
		Ok(())
	}

	fn cancel(&mut self) {
		self.status_view.clear();
		self.mode = Mode::Normal;
	}

	fn update_tree_view(&mut self, store: &Store, ret: SelRetention) -> Result<(), Error> {
		let flattree = build_flattree(self.tree_view.root_id(), store, self.tvconstr.tree_height(), self.tvconstr.tree_width())?;
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

impl BufPrint<TreeViewController, ()> for Screen {
	fn bufprint(&mut self, tvc: &TreeViewController, _: &()) -> io::Result<&mut Self> {
		match tvc.mode {
			Mode::Normal => self
				.clear()?
				.bufprint(&tvc.tree_view, &tvc.tvconstr)?
				.flush()?,
			Mode::Command(_) => self
				.clear()?
				.bufprint(&tvc.status_view, &tvc.svconstr)?
				.bufprint(&tvc.tree_view, &tvc.tvconstr)?
				.flush()?,
		}
		Ok(self)
	}
}
