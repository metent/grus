use std::io::{self, stdout};
use std::panic;
use crossterm::{cursor, terminal, ExecutableCommand};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

pub struct TermLock {
	_raw_mode: RawMode,
	_cursor_hide: CursorHide,
	_alt_screen: AltScreen,
}

impl TermLock {
	pub fn new() -> io::Result<Self> {
		Ok(TermLock {
			_raw_mode: RawMode::new()?,
			_cursor_hide: CursorHide::new()?,
			_alt_screen: AltScreen::new()?,
		})
	}
}

struct RawMode;

impl RawMode {
	fn new() -> io::Result<RawMode> {
		terminal::enable_raw_mode()?;
		Ok(RawMode)
	}
}

impl Drop for RawMode {
	fn drop(&mut self) {
		_ = terminal::disable_raw_mode();
	}
}

struct CursorHide;

impl CursorHide {
	fn new() -> io::Result<CursorHide> {
		stdout().execute(cursor::Hide)?;
		Ok(CursorHide)
	}
}

impl Drop for CursorHide {
	fn drop(&mut self) {
		_ = stdout().execute(cursor::Show);
	}
}

struct AltScreen;

impl AltScreen {
	fn new() -> io::Result<AltScreen> {
		stdout().execute(EnterAlternateScreen)?;
		Ok(AltScreen)
	}
}

impl Drop for AltScreen {
	fn drop(&mut self) {
		_ = stdout().execute(LeaveAlternateScreen);
	}
}

pub fn set_panic_hook() {
	let default_hook = panic::take_hook();
	panic::set_hook(Box::new(move |info| {
		drop(AltScreen);
		drop(CursorHide);
		drop(RawMode);
		default_hook(info);
	}))
}
