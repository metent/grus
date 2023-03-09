use std::io;
use std::path::{Path, PathBuf};
use crossterm::terminal;
use grus_lib::Store;
use crate::svc::SessionViewController;
use crate::tvc::TreeViewController;
use crate::ui::{BufPrint, Screen, SessionViewMode};

pub struct Application {
	pub store: Store,
	pub store_args: StoreArgs,
	pub screen: Screen,
	pub tvc: TreeViewController,
	pub svc: SessionViewController,
	pub view: View,
}

impl Application {
	pub fn init<P: AsRef<Path>>(path: P, n_roots: usize, export_path: P) -> Result<Self, Error> {
		let store_args = StoreArgs { n_roots, path: path.as_ref().into(), export_path: export_path.as_ref().into() };
		let store = Store::open(path, n_roots)?;
		let screen = Screen::new()?;
		let tvc = TreeViewController::new(&store)?;
		let svc = SessionViewController::new(&store)?;
		let view = View::Tree;

		Ok(Application { store, store_args, screen, tvc, svc, view })
	}

	pub fn run(mut self) -> Result<(), Error> {
		self.draw()?;

		loop {
			match match self.view {
				View::Tree => self.tvc.run(&self.store)?,
				View::Session => self.svc.run(&self.store)?,
			} {
				Action::Switch(view) => {
					self.view = view;
					self.update_view()?;
				}
				Action::Quit => break,
				Action::TaskSessions(id) => {
					self.view = View::Session;
					self.svc.change_mode(&self.store, SessionViewMode::Task(id))?;
				}
				Action::Import => {
					drop(self.store);
					std::fs::copy(&self.store_args.export_path, &self.store_args.path)?;
					self.store = Store::open(&self.store_args.path, self.store_args.n_roots)?;
					self.update_view()?;
				}
				Action::Export => {
					std::fs::copy(&self.store_args.path, &self.store_args.export_path)?;
				}
				Action::None => {}
			}
			self.draw()?;
		}

		Ok(())
	}

	fn update_view(&mut self) -> Result<(), Error> {
		match self.view {
			View::Tree => {
				let (w, h) = terminal::size()?;
				self.tvc.resize(&self.store, w, h)?;
			}
			View::Session => {
				let (w, h) = terminal::size()?;
				self.svc.resize(w, h);
				self.svc.update_session_view(&self.store)?;
			}
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

pub struct StoreArgs {
	n_roots: usize,
	path: PathBuf,
	export_path: PathBuf,
}

pub enum Action {
	Quit,
	Switch(View),
	TaskSessions(u64),
	Import,
	Export,
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
