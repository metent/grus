use std::io;
use std::path::Path;
use crossterm::terminal;
use grus_lib::Store;
use crate::svc::SessionViewController;
use crate::tvc::TreeViewController;
use crate::ui::{BufPrint, Screen, SessionViewMode};

pub struct Application {
	pub store: Store,
	pub screen: Screen,
	pub tvc: TreeViewController,
	pub svc: SessionViewController,
	pub view: View,
}

impl Application {
	pub fn init<P: AsRef<Path>>(path: P, n_roots: usize) -> Result<Self, Error> {
		let store = Store::open(path, n_roots)?;
		let screen = Screen::new()?;
		let tvc = TreeViewController::new(&store)?;
		let svc = SessionViewController::new(&store)?;
		let view = View::Tree;

		Ok(Application { store, screen, tvc, svc, view })
	}

	pub fn run(mut self) -> Result<(), Error> {
		self.draw()?;

		loop {
			match match self.view {
				View::Tree => self.tvc.run(&self.store)?,
				View::Session => self.svc.run(&self.store)?,
			} {
				Action::Switch(view) => match view {
					View::Tree => {
						self.view = view;
						let (w, h) = terminal::size()?;
						self.tvc.resize(&self.store, w, h)?;
					}
					View::Session => {
						self.view = view;
						let (w, h) = terminal::size()?;
						self.svc.resize(w, h);
						self.svc.update_session_view(&self.store)?;
					}
				}
				Action::Quit => break,
				Action::TaskSessions(id) => {
					self.view = View::Session;
					self.svc.change_mode(&self.store, SessionViewMode::Task(id))?;
				}
				Action::None => {}
			}
			self.draw()?;
		}

		Ok(())
	}

	fn draw(&mut self) -> io::Result<()> {
		match self.view {
			View::Tree => self.screen.bufprint(&self.tvc)?,
			View::Session => self.screen.bufprint(&self.svc)?,
		};
		Ok(())
	}
}

pub enum Action {
	Quit,
	Switch(View),
	TaskSessions(u64),
	None,
}

pub enum View {
	Tree,
	Session,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Store Error: {0}")]
	StoreError(#[from] sanakirja::Error),
	#[error("IO Error: {0}")]
	IoError(#[from] io::Error),
}
