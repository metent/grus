use std::fs;
use grus::app::{Application, Error};
use grus::actions::Actions;
use grus::node::NodeData;

#[test]
fn add_child() -> Result<(), Error> {
	test_action("add_child", |mut app| {
		"one".chars().for_each(|c| app.status_view.command.push(c));
		app.add_child()?;
		app.tree_view.move_sel_down();
		"two".chars().for_each(|c| app.status_view.command.push(c));
		app.add_child()?;

		let reader = app.store.reader()?;
		let mut iter = reader.children(0)?;

		let (id, data) = iter.next().unwrap()?;
		assert_eq!(id, 1);
		let NodeData { name, .. } = bincode::deserialize(data)?;
		assert_eq!(name, "one");
		assert!(iter.next().is_none());

		let mut iter = reader.children(1)?;

		let (id, data) = iter.next().unwrap()?;
		assert_eq!(id, 2);
		let NodeData { name, .. } = bincode::deserialize(data)?;
		assert_eq!(name, "two");
		assert!(iter.next().is_none());

		Ok(())
	})
}

fn test_action(name: &str, action: fn(Application) -> Result<(), Error>) -> Result<(), Error> {
	let res_holder = ResourceHolder::new(name);
	let app = Application::init(&res_holder.store, 2)?;
	action(app)
}

struct ResourceHolder {
	store: String,
}

impl ResourceHolder {
	fn new(name: &str) -> Self {
		ResourceHolder { store: env!("CARGO_TARGET_TMPDIR").to_string() + "/" + name }
	}
}

impl Drop for ResourceHolder {
	fn drop(&mut self) {
		_ = fs::remove_file(&self.store);
		_ = fs::remove_file(self.store.clone() + ".lock0");
		_ = fs::remove_file(self.store.clone() + ".lock1");
	}
}
