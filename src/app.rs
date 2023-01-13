use std::io;
use std::path::Path;
use crossterm::event::{self, KeyCode, Event};
use crate::actions::{Actions, build_flattree};
use crate::node::{NodeData, Priority};
use crate::store::Store;
use crate::ui::{Screen, status::StatusView, tree::TreeView};
use crate::ui::BufPrint;

pub struct Application {
	pub store: Store,
	pub screen: Screen,
	pub status_view: StatusView,
	pub tree_view: TreeView,
	pub mode: Mode,
	pub root_id: u64,
	pub stack: Vec<u64>,
}

impl Application {
	pub fn init<P: AsRef<Path>>(path: P, n_roots: usize) -> Result<Self, Error> {
		let root_data = bincode::serialize(&NodeData::with_name("/"))?;
		let store = Store::open(path, &root_data, n_roots)?;
		let screen = Screen::init()?;
		let status_view = StatusView::new();
		let flattree = build_flattree(0, &store, screen.tree_height().into(), screen.tree_width().into())?;
		let tree_view = TreeView::new(flattree);
		let mode = Mode::Normal;
		let root_id = 0;
		let stack = Vec::new();

		Ok(Application { store, screen, status_view, tree_view, mode, root_id, stack })
	}

	pub fn run(mut self) -> Result<(), Error> {
		self.draw()?;

		loop {
			match event::read()? {
				Event::Key(kev) => match self.mode {
					Mode::Normal => match kev.code {
						KeyCode::Char('q') => break,
						KeyCode::Char('j') => self.tree_view.move_sel_down(),
						KeyCode::Char('k') => self.tree_view.move_sel_up(),
						KeyCode::Char('h') => self.move_out()?,
						KeyCode::Char('l') => self.move_into()?,
						KeyCode::Char('a') => self.enter_command_mode(CommandType::AddChild),
						KeyCode::Char('r') => self.enter_command_mode(CommandType::Rename),
						KeyCode::Char('x') => self.enter_command_mode(CommandType::SetDueDate),
						KeyCode::Char('X') => self.unset_due_date()?,
						KeyCode::Char('H') => self.set_priority(Priority::High)?,
						KeyCode::Char('M') => self.set_priority(Priority::Medium)?,
						KeyCode::Char('L') => self.set_priority(Priority::Low)?,
						KeyCode::Char('N') => self.set_priority(Priority::None)?,
						KeyCode::Char('d') => self.delete()?,
						_ => {},
					}
					Mode::Command(cmd) => match kev.code {
						KeyCode::Enter => {
							match cmd {
								CommandType::AddChild => self.add_child()?,
								CommandType::Rename => self.rename()?,
								CommandType::SetDueDate => self.set_due_date()?,
							}
						}
						KeyCode::Char(c) => self.status_view.pushc(c),
						KeyCode::Backspace => self.status_view.back(),
						KeyCode::Esc => self.cancel(),
						_ => {},
					}
				}
				Event::Resize(w, h) => self.resize(w, h)?,
				_ => {},
			}
			self.draw()?;
		}

		Ok(())
	}

	fn draw(&mut self) -> io::Result<()> {
		match self.mode {
			Mode::Normal => self.screen
				.clear()?
				.bufprint(&self.tree_view)?
				.flush()?,
			Mode::Command(_) => self.screen
				.clear()?
				.bufprint(&self.status_view)?
				.bufprint(&self.tree_view)?
				.flush()?,
		}
		Ok(())
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
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Store Error: {0}")]
	StoreError(#[from] sanakirja::Error),
	#[error("IO Error: {0}")]
	IoError(#[from] io::Error),
	#[error("Bincode Error: {0}")]
	BincodeError(#[from] bincode::Error),
}
