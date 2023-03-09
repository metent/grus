use std::fs;
use grus::app::{Application, Error};
use grus::global::{TermLock, set_panic_hook};

fn main() -> Result<(), Error> {
	let Some(mut data_path) = dirs::data_dir() else {
		eprintln!("Error: Data directory could not be determined.");
		return Ok(())
	};
	data_path.push("grus");
	fs::create_dir_all(&data_path)?;
	data_path.push("tasks");

	let Some(mut export_path) = dirs::home_dir() else {
		eprintln!("Error: Home directory could not be determined.");
		return Ok(())
	};
	export_path.push("sync");
	fs::create_dir_all(&export_path)?;
	export_path.push("tasks");

	let _lock = TermLock::new()?;
	set_panic_hook();
	Application::init(data_path, 2, export_path)?.run()
}
