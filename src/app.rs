use std::io;
use std::path::Path;
use crate::node::NodeData;
use crate::store::Store;
use crate::tvc::TreeViewController;
use crate::ui::Screen;
use crate::ui::BufPrint;

pub struct Application {
	pub store: Store,
	pub screen: Screen,
	pub tvc: TreeViewController,
}

impl Application {
	pub fn init<P: AsRef<Path>>(path: P, n_roots: usize) -> Result<Self, Error> {
		let root_data = bincode::serialize(&NodeData::with_name("/"))?;
		let store = Store::open(path, &root_data, n_roots)?;
		let screen = Screen::new()?;
		let tvc = TreeViewController::new(&store)?;

		Ok(Application { store, screen, tvc })
	}

	pub fn run(mut self) -> Result<(), Error> {
		self.draw()?;

		loop {
			match self.tvc.run(&self.store)? {
				Action::Switch(_) => {},
				Action::Quit => break,
				Action::None => {},
			}
			self.draw()?;
		}

		Ok(())
	}

	fn draw(&mut self) -> io::Result<()> {
		self.screen.bufprint(&self.tvc, &())?;
		Ok(())
	}
}

pub enum Action {
	Quit,
	Switch(View),
	None,
}

pub enum View {
	Tree
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
	AddSession,
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
