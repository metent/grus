use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use chrono::naive::Days;
use grus_lib::types::Session;
use winnow::{IResult, Parser};
use winnow::branch::alt;
use winnow::bytes::{tag, tag_no_case, take, take_until1, take_while0, take_while1};
use winnow::error::{ErrMode, Error, ErrorKind, FinishIResult};
use winnow::sequence::separated_pair;
use winnow::stream::AsChar;

pub fn parse_session(s: &str) -> Result<Session, Error<&str>> {
	alt((
		separated_pair(datetime, tag(" to "), datetime)
			.map(|(dt1, dt2)| Session { start: dt1, end: dt2 }),
		separated_pair(datetime, tag(" to "), time)
			.map(|(dt, time)| Session { start: dt, end: dt.date().and_time(time) }),
		separated_pair(time, tag(" to "), datetime).map(|(time, dt)| Session {
			start: Local::now().date_naive().and_time(time),
			end: dt
		}),
		separated_pair(time, tag(" to "), time).map(|(t1, t2)| Session {
			start: Local::now().date_naive().and_time(t1),
			end: Local::now().date_naive().and_time(t2)
		}),
	))(s).finish()
}

pub fn parse_datetime(s: &str) -> Result<NaiveDateTime, Error<&str>> {
	alt((
		datetime,
		time.map(|time| NaiveDateTime::new(Local::now().date_naive(), time)),
		date.map(|date| NaiveDateTime::new(date, NaiveTime::default())),
	))(s).finish()
}

fn datetime(s: &str) -> IResult<&str, NaiveDateTime> {
	let (s, date) = date(s)?;
	let (s, _) = take_while0(AsChar::is_space)(s)?;
	let (s, time) = time(s)?;
	Ok((s, NaiveDateTime::new(date, time)))
}

fn time(s: &str) -> IResult<&str, NaiveTime> {
	alt((
		proper_time,
		quick_time,
	))(s)
}

fn proper_time(s: &str) -> IResult<&str, NaiveTime> {
	let (s, hour) = take_until1(":")(s)?;
	let (s, _) = tag(":")(s)?;
	let (s, minute) = take(2usize)(s)?;
	let (s, _) = take_while0(AsChar::is_space)(s)?;
	let (s, delta) = alt((
		alt((tag("am"), tag("AM"))).map(|_| 0),
		alt((tag("pm"), tag("PM"))).map(|_| 12),
	))(s)?;
	let mut h = hour.parse().map_err(|_| ErrMode::Cut(Error::new(s, ErrorKind::Digit)))?;
	if h > 12 { return Err(ErrMode::Cut(Error::new(s, ErrorKind::Digit))) }
	if h == 12 { h = 0 }
	h += delta;
	let m = minute.parse().map_err(|_| ErrMode::Cut(Error::new(s, ErrorKind::Digit)))?;
	Ok((s, NaiveTime::from_hms_opt(h, m, 0).ok_or(ErrMode::Cut(Error::new(s, ErrorKind::Digit)))?))
}

fn quick_time(s: &str) -> IResult<&str, NaiveTime> {
	let (s, hour) = take_while1(AsChar::is_dec_digit)(s)?;
	let (s, _) = take_while0(AsChar::is_space)(s)?;
	let (s, delta) = alt((
		alt((tag("am"), tag("AM"))).map(|_| 0),
		alt((tag("pm"), tag("PM"))).map(|_| 12),
	))(s)?;
	let mut h = hour.parse().map_err(|_| ErrMode::Cut(Error::new(s, ErrorKind::Digit)))?;
	if h > 12 { return Err(ErrMode::Cut(Error::new(s, ErrorKind::Digit))) }
	if h == 12 { h = 0 }
	h += delta;
	Ok((s, NaiveTime::from_hms_opt(h, 0, 0).ok_or(ErrMode::Cut(Error::new(s, ErrorKind::Digit)))?))
}

fn date(s: &str) -> IResult<&str, NaiveDate> {
	alt((
		tag_no_case("today").map(|_| Local::now().date_naive()),
		tag_no_case("yesterday").map(|_| Local::now().date_naive() - Days::new(1)),
		alt((tag_no_case("tmrw"), tag_no_case("tomorrow")))
			.map(|_| Local::now().date_naive() + Days::new(1)),
		weekday,
		ddmmyyyy,
	))(s)
}

fn weekday(s: &str) -> IResult<&str, NaiveDate> {
	let (s, weekday) = alt((
		alt((tag_no_case("mon"), tag_no_case("monday"))).map(|_| Weekday::Mon),
		alt((tag_no_case("tue"), tag_no_case("tuesday"))).map(|_| Weekday::Tue),
		alt((tag_no_case("wed"), tag_no_case("wednesday"))).map(|_| Weekday::Wed),
		alt((tag_no_case("thu"), tag_no_case("thursday"))).map(|_| Weekday::Thu),
		alt((tag_no_case("fri"), tag_no_case("friday"))).map(|_| Weekday::Fri),
		alt((tag_no_case("sat"), tag_no_case("saturday"))).map(|_| Weekday::Sat),
		alt((tag_no_case("sun"), tag_no_case("sunday"))).map(|_| Weekday::Sun),
	))(s)?;
	let today = Local::now().date_naive();
	let delta = (weekday.num_days_from_monday() + 7 - today.weekday().num_days_from_monday()) % 7;
	Ok((s, today + Days::new(delta.into())))
}

fn ddmmyyyy(s: &str) -> IResult<&str, NaiveDate> {
	let (s, day) = take(2usize)(s)?;
	let (s, _) = tag("/")(s)?;
	let (s, month) = take(2usize)(s)?;
	let (s, _) = tag("/")(s)?;
	let (s, year) = take(4usize)(s)?;
	Ok((s, NaiveDate::from_ymd_opt(
		year.parse().map_err(|_| ErrMode::Cut(Error::new(s, ErrorKind::Digit)))?,
		month.parse().map_err(|_| ErrMode::Cut(Error::new(s, ErrorKind::Digit)))?,
		day.parse().map_err(|_| ErrMode::Cut(Error::new(s, ErrorKind::Digit)))?,
	).ok_or(ErrMode::Cut(Error::new(s, ErrorKind::Digit)))?))
}
