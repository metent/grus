use std::fs;
use grus::app::{Application, Error};
use grus::global::{TermLock, set_panic_hook};

fn main() -> Result<(), Error> {
	let Some(mut pathbuf) = dirs::data_dir() else {
		eprintln!("Error: Data directory could not be determined.");
		return Ok(())
	};
	pathbuf.push("grus");
	fs::create_dir_all(&pathbuf)?;
	pathbuf.push("tasks");

	let _lock = TermLock::new()?;
	set_panic_hook();
	Application::init(pathbuf, 2)?.run()
}
