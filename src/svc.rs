use std::io;
use crossterm::event::{self, KeyCode, Event};
use crate::app::{Action, Error, Mode, View};
use crate::node::{wrap_text, Displayable, NodeData};
use crate::store::Store;
use crate::ui::{BufPrint, Screen, SessionViewConstraints, StatusViewConstraints};
use crate::ui::session::{Item, SessionView};
use crate::ui::status::StatusView;

pub struct SessionViewController {
	session_view: SessionView,
	ssvconstr: SessionViewConstraints,
	status_view: StatusView,
	svconstr: StatusViewConstraints,
	mode: Mode,
}

impl SessionViewController {
	pub fn new(store: &Store) -> Result<Self, Error> {
		let ssvconstr = SessionViewConstraints::new()?;
		let mut svc = SessionViewController {
			session_view: SessionView::new(Vec::new(), ssvconstr.session_height()),
			ssvconstr,
			status_view: StatusView::new(),
			svconstr: StatusViewConstraints::new()?,
			mode: Mode::Normal,
		};
		svc.update_session_view(store)?;
		Ok(svc)
	}

	pub fn run(&mut self, store: &Store) -> Result<Action, Error> {
		match event::read()? {
			Event::Key(kev) => match self.mode {
				Mode::Normal => match kev.code {
					KeyCode::Char('q') => return Ok(Action::Quit),
					KeyCode::Char('j') | KeyCode::Down => self.session_view.cursor_down(),
					KeyCode::Char('k') | KeyCode::Up => self.session_view.cursor_up(),
					KeyCode::Char('d') => self.delete(store)?,
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
		self.ssvconstr.update(w, h);
		self.svconstr.update(w, h);
		self.session_view.resize(self.ssvconstr.session_height(), self.ssvconstr.tasks_width(), self.ssvconstr.session_width());
	}

	pub fn update_session_view(&mut self, store: &Store) -> Result<(), Error> {
		let mut items = Vec::new();
		let reader = store.reader()?;
		for entry in reader.all_sessions()? {
			let (&session, &id) = entry?;
			let Some(data) = reader.read(id)? else { continue };
			let NodeData { name, .. } = bincode::deserialize(data)?;
			let name_splits = wrap_text(&name, self.ssvconstr.tasks_width());
			let session_text = format!("{}", Displayable(Some(session)));
			let session_splits = wrap_text(&session_text, self.ssvconstr.session_width());
			items.push(Item { session, id, name: name.into(), name_splits, session_text, session_splits });
		}
		self.session_view = SessionView::new(items, self.ssvconstr.session_height());
		Ok(())
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
		self.mode = Mode::Normal;
	}
}

impl BufPrint<SessionViewController, ()> for Screen {
	fn bufprint(&mut self, svc: &SessionViewController, _: &()) -> io::Result<&mut Self> {
		match svc.mode {
			Mode::Normal => self
				.clear()?
				.bufprint(&svc.session_view, &svc.ssvconstr)?
				.flush()?,
			Mode::Command(_) => self
				.clear()?
				.bufprint(&svc.status_view, &svc.svconstr)?
				.bufprint(&svc.session_view, &svc.ssvconstr)?
				.flush()?,
		}
		Ok(self)
	}
}
