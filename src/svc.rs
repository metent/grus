use std::io;
use crossterm::terminal;
use crossterm::event::{self, KeyCode, Event};
use grus_lib::Store;
use grus_lib::reader::StoreReader;
use crate::app::{Action, Error, View};
use crate::node::{wrap_text, Displayable};
use crate::ui::{BufPrint, Screen, SessionViewMode};
use crate::ui::session::{Item, SessionView};
use crate::ui::status::{Mode, StatusView};

pub struct SessionViewController {
	session_view: SessionView,
	status_view: StatusView<{View::Session as usize}>,
}

impl SessionViewController {
	pub fn new(store: &Store) -> Result<Self, Error> {
		let mut svc = SessionViewController {
			session_view: SessionView::new(Vec::new())?,
			status_view: StatusView::new()?,
		};
		svc.update_session_view(store)?;
		Ok(svc)
	}

	pub fn run(&mut self, store: &Store) -> Result<Action, Error> {
		match event::read()? {
			Event::Key(kev) => match self.status_view.mode {
				Mode::Normal => match kev.code {
					KeyCode::Char('q') => return Ok(Action::Quit),
					KeyCode::Char('j') | KeyCode::Down => self.session_view.cursor_down(),
					KeyCode::Char('k') | KeyCode::Up => self.session_view.cursor_up(),
					KeyCode::Char('D') => self.delete(store)?,
					KeyCode::Char('v') if self.session_view.constr.mode == SessionViewMode::Normal
					=> if let Some((id, _)) = self.session_view.session_and_id() {
						self.change_mode(store, SessionViewMode::Task(id))?;
					}
					KeyCode::Char('v') if self.session_view.constr.mode != SessionViewMode::Normal
						=> self.change_mode(store, SessionViewMode::Normal)?,
					KeyCode::Char('1') => return Ok(Action::Switch(View::Tree)),
					_ => {},
				}
				Mode::Command(_) => match kev.code {
					KeyCode::Char(c) => self.status_view.insert(c),
					KeyCode::Backspace => _ = self.status_view.delete(),
					KeyCode::Left => self.status_view.move_left(),
					KeyCode::Right => self.status_view.move_right(),
					KeyCode::Esc => self.cancel(),
					_ => {},
				}
			}
			Event::Resize(w, h) => self.resize(w, h),
			_ => {}
		}
		Ok(Action::None)
	}

	pub fn resize(&mut self, w: u16, h: u16) {
		self.session_view.constr.update(w, h);
		self.status_view.constr.update(w, h);
		self.session_view.resize(self.session_view.constr.tasks_width(), self.session_view.constr.session_width());
	}

	pub fn update_session_view(&mut self, store: &Store) -> Result<(), Error> {
		let reader = SessionViewReader {
			reader: store.reader()?,
			tasks_width: self.session_view.constr.tasks_width(),
			session_width: self.session_view.constr.session_width(),
		};

		match self.session_view.constr.mode {
			SessionViewMode::Normal => {
				let items = reader.get_items()?;
				self.session_view.reset(items);
			}
			SessionViewMode::Task(id) => {
				let items = reader.get_task_items(id)?;
				self.session_view.reset(items);
			}
		}
		Ok(())
	}

	pub fn change_mode(&mut self, store: &Store, mode: SessionViewMode) -> Result<(), Error> {
		self.session_view.constr.mode = mode;
		let (w, h) = terminal::size()?;
		self.session_view.constr.update(w, h);
		self.status_view.constr.update(w, h);
		self.session_view = SessionView::new(Vec::new())?;
		self.update_session_view(store)
	}

	fn delete(&mut self, store: &Store) -> Result<(), Error> {
		let Some((id, session)) = self.session_view.session_and_id() else { return Ok(()) };

		let mut writer = store.writer()?;
		writer.delete_session(id, session)?;
		writer.commit()?;

		self.update_session_view(store)?;
		Ok(())
	}

	fn cancel(&mut self) {
		self.status_view.clear();
		self.status_view.mode = Mode::Normal;
	}
}

struct SessionViewReader<'store> {
	reader: StoreReader<'store>,
	tasks_width: usize,
	session_width: usize,
}

impl<'store> SessionViewReader<'store> {
	fn get_items(&self) -> Result<Vec<Item>, Error> {
		let mut items = Vec::new();
		for entry in self.reader.all_sessions()? {
			let (&session, &id) = entry?;
			let Some(name) = self.reader.name(id)? else { continue };
			let name_splits = wrap_text(&name, self.tasks_width);
			let session_text = format!("{}", Displayable(Some(session)));
			let session_splits = wrap_text(&session_text, self.session_width);
			items.push(Item { session, id, name: name.into(), name_splits, session_text, session_splits });
		}
		Ok(items)
	}

	fn get_task_items(&self, id: u64) -> Result<Vec<Item>, Error> {
		let mut items = Vec::new();
		for entry in self.reader.sessions(id)? {
			let (_, &session) = entry?;
			let Some(name) = self.reader.name(id)? else { continue };
			let name_splits = wrap_text(&name, self.tasks_width);
			let session_text = format!("{}", Displayable(Some(session)));
			let session_splits = wrap_text(&session_text, self.session_width);
			items.push(Item { session, id, name: name.into(), name_splits, session_text, session_splits });
		}
		Ok(items)
	}
}

impl BufPrint<SessionViewController> for Screen {
	fn bufprint(&mut self, svc: &SessionViewController) -> io::Result<&mut Self> {
		self
			.clear()?
			.bufprint(&svc.status_view)?
			.bufprint(&svc.session_view)?
			.flush()?;
		Ok(self)
	}
}
