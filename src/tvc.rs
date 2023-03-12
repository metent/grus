use std::collections::HashSet;
use std::io;
use crossterm::event::{self, KeyCode, Event};
use grus_lib::Store;
use grus_lib::reader::StoreReader;
use crate::app::{Action, Error, View};
use crate::flattree::{FlatTreeBuilder, FlatTreeState};
use crate::node::{Displayable, Node, Priority, wrap_text};
use crate::parser::{parse_datetime, parse_session};
use crate::ui::{BufPrint, Screen};
use crate::ui::tree::TreeView;
use crate::ui::status::{CommandType, Mode, StatusView};

pub struct TreeViewController {
	tree_view: TreeView,
	status_view: StatusView<{View::Tree as usize}>,
}

impl TreeViewController {
	pub fn new(store: &Store) -> Result<Self, Error> {
		let mut tvc = TreeViewController {
			tree_view: TreeView::new(Vec::new())?,
			status_view: StatusView::new()?,
		};
		tvc.update_tree_view(store)?;
		Ok(tvc)
	}

	pub fn run(&mut self, store: &Store) -> Result<Action, Error> {
		match event::read()? {
			Event::Key(kev) => match self.status_view.mode {
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
					KeyCode::Char('D') => self.delete(store)?,
					KeyCode::Char('v') => if let Some(node) = self.tree_view.cursor_node() {
						return Ok(Action::TaskSessions(node.id));
					},
					KeyCode::Char('2') => return Ok(Action::Switch(View::Session)),
					KeyCode::Char('I') => return Ok(Action::Import),
					KeyCode::Char('E') => return Ok(Action::Export),
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
		self.tree_view.constr.update(w, h);
		self.status_view.constr.update(w, h);
		self.update_tree_view(store)?;
		Ok(())
	}

	fn enter_command_mode(&mut self, cmd: CommandType) {
		let Some(node) = self.tree_view.cursor_node() else { return };

		self.status_view.mode = Mode::Command(cmd);
		match cmd {
			CommandType::Rename => self.status_view.set_input(&node.name),
			_ => {},
		};
	}

	fn add_child(&mut self, store: &Store) -> Result<(), Error> {
		let mut selections = self.tree_view.selection_ids();
		let Some(&first) = selections.next() else { return Ok(()) };

		let mut writer = store.writer()?;
		let id = writer.add_child(first, self.status_view.input())?;

		for &pid in selections {
			writer.share(id, pid)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(store)?;

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
		self.update_tree_view(store)?;
		Ok(())
	}

	fn rename(&mut self, store: &Store) -> Result<(), Error> {
		let mut writer = store.writer()?;
		let name = self.status_view.input();
		for &id in self.tree_view.selection_ids() {
			writer.rename(id, name)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(store)?;

		self.cancel();
		Ok(())
	}

	fn set_due_date(&mut self, store: &Store) -> Result<(), Error> {
		let mut writer = store.writer()?;
		let Ok(due_date) = parse_datetime(self.status_view.input()) else {
			return Ok(());
		};
		for &id in self.tree_view.selection_ids() {
			writer.set_due_date(id, due_date)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(store)?;

		self.cancel();
		Ok(())
	}

	fn unset_due_date(&mut self, store: &Store) -> Result<(), Error> {
		let mut writer = store.writer()?;
		for &id in self.tree_view.selection_ids() {
			writer.unset_due_date(id)?;
		}
		writer.commit()?;

		self.tree_view.clear_selections();
		self.update_tree_view(store)?;
		Ok(())
	}

	fn add_session(&mut self, store: &Store) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		let mut writer = store.writer()?;

		let Ok(session) = parse_session(&self.status_view.input()) else { return Ok(()) };
		writer.add_session(node.id, &session)?;
		writer.commit()?;

		self.update_tree_view(store)?;
		self.cancel();
		Ok(())
	}

	fn priority_up(&mut self, store: &Store) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		if self.tree_view.is_cursor_at_root() { return Ok(()) };

		let mut writer = store.writer()?;
		writer.move_up(node.pid, node.id)?;
		writer.commit()?;

		self.update_tree_view(store)?;
		Ok(())
	}

	fn priority_down(&mut self, store: &Store) -> Result<(), Error> {
		let Some(node) = self.tree_view.cursor_node() else { return Ok(()) };

		if self.tree_view.is_cursor_at_root() { return Ok(()) };

		let mut writer = store.writer()?;
		writer.move_down(node.pid, node.id)?;
		writer.commit()?;

		self.update_tree_view(store)?;
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
		self.update_tree_view(store)?;
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
		self.update_tree_view(store)?;
		Ok(())
	}

	fn move_into(&mut self, store: &Store) -> Result<(), Error> {
		self.tree_view.move_into();
		self.update_tree_view(store)?;
		Ok(())
	}

	fn move_out(&mut self, store: &Store) -> Result<(), Error> {
		self.tree_view.move_out();
		self.update_tree_view(store)?;
		Ok(())
	}

	fn cancel(&mut self) {
		self.status_view.clear();
		self.status_view.mode = Mode::Normal;
	}

	fn update_tree_view(&mut self, store: &Store) -> Result<(), Error> {
		let flattree = TreeViewReader {
			reader: store.reader()?,
			height: self.tree_view.constr.tree_height(),
			tasks_width: self.tree_view.constr.tree_width(),
			session_width: self.tree_view.constr.session_width(),
			due_date_width: self.tree_view.constr.due_date_width(),
		}.build_flattree(self.tree_view.root_id)?;
		self.tree_view.reset(flattree);
		Ok(())
	}
}

struct TreeViewReader<'store> {
	reader: StoreReader<'store>,
	height: usize,
	tasks_width: usize,
	session_width: usize,
	due_date_width: usize,
}

impl<'store> TreeViewReader<'store> {
	fn build_flattree(&self, id: u64) -> Result<Vec<Node<'static>>, Error> {
		if self.tasks_width <= 1 { return Ok(Vec::new()) };

		let root = self.get_node(id, id, 0)?;
		if root.name_splits.len() - 1 > self.height { return Ok(Vec::new()) };
		let mut builder = FlatTreeBuilder::new(root, self.height);
		let mut ids = HashSet::new();
		loop {
			match builder.step() {
				FlatTreeState::Build => continue,
				FlatTreeState::Refill => {
					for i in builder.fill_range() {
						let id = builder.id(i);
						if ids.insert(id) {
							builder.fill(self.get_children(id, builder.depth(i))?, i);
						}
					}
					builder.finish_fill();
				}
				FlatTreeState::Done => return Ok(builder.finish()),
			}
		}
	}

	fn get_children(&self, pid: u64, mut depth: usize) -> Result<Vec<Node<'static>>, Error> {
		depth += 1;
		let mut children = Vec::new();
		for id in self.reader.child_ids(pid)? {
			if 2 * depth + 1 >= self.tasks_width { continue };
			children.push(self.get_node(pid, id?, depth)?);
		}
		for i in 0..children.len() {
			children[i].priority = Priority {
				det: i as u64,
				total: children.len() as u64,
			};
		}
		Ok(children)
	}

	fn get_node(&self, pid: u64, id: u64, depth: usize) -> Result<Node<'static>, Error> {
		let name = self.reader.name(id)?.unwrap().to_string();
		let due_date = self.reader.due_date(id)?;
		let width = self.tasks_width - 2 * depth - 1;
		let name_splits = wrap_text(&name, width);
		let session = self.reader.first_session(id)?;
		let session_text = format!("{}", Displayable(session));
		let session_splits = wrap_text(&session_text, self.session_width);
		let due_date_text = format!("{}", Displayable(due_date));
		let due_date_splits = wrap_text(&due_date_text, self.due_date_width);
		Ok(Node {
			id,
			pid,
			depth,
			name: name.into(),
			due_date,
			session,
			priority: Priority::default(),
			name_splits,
			session_text,
			session_splits,
			due_date_text,
			due_date_splits,
		})
	}
}

impl BufPrint<TreeViewController> for Screen {
	fn bufprint(&mut self, tvc: &TreeViewController) -> io::Result<&mut Self> {
		self
			.clear()?
			.bufprint(&tvc.status_view)?
			.bufprint(&tvc.tree_view)?
			.flush()?;
		Ok(self)
	}
}
