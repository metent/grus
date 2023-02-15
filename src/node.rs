use std::borrow::Cow;
use std::cmp::max;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use chrono::{Datelike, NaiveDateTime, Local};
use interim::{parse_date_string, DateError, Dialect};
use crate::store::Session;

#[cfg_attr(test, derive(Clone, Debug, PartialEq))]
pub struct Node<'a> {
	pub id: u64,
	pub pid: u64,
	pub depth: usize,
	pub name: Cow<'a, str>,
	pub due_date: Option<NaiveDateTime>,
	pub session: Option<Session>,
	pub priority: Priority,
	pub name_splits: Vec<usize>,
	pub session_text: String,
	pub session_splits: Vec<usize>,
	pub due_date_text: String,
	pub due_date_splits: Vec<usize>,
}

impl<'a> Node<'a> {
	pub fn name_splits(&self) -> impl Iterator<Item = &str> {
		self.name_splits.windows(2).map(|w| &self.name[w[0]..w[1]])
	}

	pub fn session_splits(&self) -> impl Iterator<Item = &str> {
		self.session_splits.windows(2).map(|w| &self.session_text[w[0]..w[1]])
	}

	pub fn due_date_splits(&self) -> impl Iterator<Item = &str> {
		self.due_date_splits.windows(2).map(|w| &self.due_date_text[w[0]..w[1]])
	}

	pub fn height(&self) -> usize {
		max(
			self.name_splits.len() - 1,
			max(self.session_splits.len() - 1, self.due_date_splits.len() - 1)
		)
	}
}

impl FromStr for Session {
	type Err = DateError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.split_once(" to ") {
			Some((s, e)) => Ok(Session {
				start: parse_date_string(s, Local::now(), Dialect::Uk)?.naive_local(),
				end: parse_date_string(e, Local::now(), Dialect::Uk)?.naive_local(),
			}),
			None => Err(DateError::MissingDate),
		}
	}
}

#[derive(Copy, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Priority {
	pub det: u64,
	pub total: u64,
}

impl Default for Priority {
	fn default() -> Self {
		Priority {
			det: 0,
			total: 1,
		}
	}
}

pub struct Displayable<T>(pub Option<T>);

impl Display for Displayable<NaiveDateTime> {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		let Displayable(Some(dt)) = self else { return Ok(()) };
		let now = Local::now().naive_local();

		if dt.year() != now.year() {
			write!(f, "{}", dt.format("%e %b %Y %-I:%M %p"))
		} else if dt.iso_week() != now.iso_week() {
			write!(f, "{}", dt.format("%e %b %-I:%M %p"))
		} else if dt.day() != now.day() {
			write!(f, "{}", dt.format("%A %-I:%M %p"))
		} else {
			write!(f, "{}", dt.format("%-I:%M %p"))
		}
	}
}

impl Display for Displayable<Session> {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		let Displayable(Some(session)) = self else { return Ok(()) };

		write!(f, "{} to {}", Displayable(Some(session.start)), Displayable(Some(session.end)))
	}
}

pub fn wrap_text(text: &str, w: usize) -> Vec<usize> {
	if w == 0 { return vec![0, 0] };
	let mut splits = vec![];
	let mut i = 0;
	let mut beg = 0;
	let mut alt_beg = 0;
	let mut in_a_word = false;
	let mut long_word = false;
	let mut d = 0;

	for (j, (pos, ch)) in text.char_indices().chain(([(text.len(), ' ')]).into_iter()).enumerate() {
		let diff = (j + d) / w - (i + d) / w;
		if ch == ' ' {
			if in_a_word {
				if j - i == w && !long_word {
					if splits.last() != Some(&i) {
						splits.push(i);
					}
					d += w - (i + d) % w;
				}

				if (j + d) % w == 0 {
					splits.push(pos);
					i = j;
					beg = pos;
				} else if diff > 0 {
					if !long_word {
						splits.push(beg);
						d += w - (i + d) % w;
					} else {
						splits.push(alt_beg);
					}
				}
				in_a_word = false;
				long_word = false;
			} else {
				if (j + d) % w == 0 {
					splits.push(pos);
					i = j;
					beg = pos;
				}
			}
		} else {
			if !in_a_word {
				if (j + d) % w == 0 {
					splits.push(pos);
				}
				i = j;
				beg = pos;
				in_a_word = true;
			} else {
				if (j + d) % w == 0 {
					alt_beg = pos;
				}
				if j - i == w {
					splits.push(alt_beg);
					i = j;
					beg = pos;
					alt_beg = pos;
					long_word = true;
				}
			}
		}
	}

	if text.len() > 0 && splits[splits.len() - 1] != text.len() {
		splits.push(text.len());
	}

	splits
}

#[cfg(test)]
mod tests {
	use super::wrap_text;

	#[test]
	fn wrap_text_test() {
		let expected = &[
			"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor ",
			"incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis ",
			"nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. ",
			"Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu ",
			"fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in ",
			"culpa qui officia deserunt mollit anim id est laborum.",
		];
		let text = expected.concat();

		let result: Vec<_> = wrap_text(&text, 80).windows(2).map(|w| &text[w[0]..w[1]]).collect();
		assert_eq!(result, expected);

		let expected = &["   ", "   ", "   ", "  "];
		let text = expected.concat();

		let result: Vec<_> = wrap_text(&text, 3).windows(2).map(|w| &text[w[0]..w[1]]).collect();
		assert_eq!(result, expected);

		let expected = &["###", "###", "###"];
		let text = expected.concat();

		let result: Vec<_> = wrap_text(&text, 3).windows(2).map(|w| &text[w[0]..w[1]]).collect();
		assert_eq!(result, expected);

		let expected = &[" ##", "###", "###", "#"];
		let text = expected.concat();

		let result: Vec<_> = wrap_text(&text, 3).windows(2).map(|w| &text[w[0]..w[1]]).collect();
		assert_eq!(result, expected);

		let expected = &["  #", "###", "###", "##"];
		let text = expected.concat();

		let result: Vec<_> = wrap_text(&text, 3).windows(2).map(|w| &text[w[0]..w[1]]).collect();
		assert_eq!(result, expected);

		let expected = &[" ", "###"];
		let text = expected.concat();

		let result: Vec<_> = wrap_text(&text, 3).windows(2).map(|w| &text[w[0]..w[1]]).collect();
		assert_eq!(result, expected);

		let expected = &[" ", "###", " "];
		let text = expected.concat();

		let result: Vec<_> = wrap_text(&text, 3).windows(2).map(|w| &text[w[0]..w[1]]).collect();
		assert_eq!(result, expected);

		let expected = &["###", "#  ", " ", "###", "   ", " ", "###"];
		let text = expected.concat();

		let result: Vec<_> = wrap_text(&text, 3).windows(2).map(|w| &text[w[0]..w[1]]).collect();
		assert_eq!(result, expected);
	}
}
