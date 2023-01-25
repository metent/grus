use std::io::{Error as IoError, ErrorKind};
use std::path::Path;
use sanakirja::{Commit, Env, Error, LoadPage, MutTxn, RootDb, Storable, Txn};
use sanakirja::btree::{self, Db, UDb};
use crate::node::Session;

type LinksDb = Db<u64, u64>;
type RLinksDb = Db<u64, RTriple>;
type NodesDb = UDb<u64, [u8]>;
type SessionsDb = Db<u64, Session>;
type RSessionsDb = Db<Session, u64>;

const ID_SQ: usize = 0;
const DB_LINKS: usize = 1;
const DB_RLINKS: usize = 2;
const DB_NODES: usize = 3;
const DB_SESSIONS: usize = 4;
const DB_RSESSIONS: usize = 5;

pub struct Store {
	env: Env
}

impl Store {
	pub fn open<P: AsRef<Path>>(
		path: P,
		root_data: &[u8],
		n_roots: usize
	) -> Result<Self, Error> {
		let store = Store { env: Env::new(path, 1 << 14, n_roots)? };
		store.create_base(root_data)?;
		Ok(store)
	}

	pub fn reader(&self) -> Result<StoreReader, Error> {
		let txn = Env::txn_begin(&self.env)?;
		let id = txn.root(ID_SQ);
		let links = txn.root_db(DB_LINKS).ok_or_else(invalid_data_error)?;
		let rlinks = txn.root_db(DB_RLINKS).ok_or_else(invalid_data_error)?;
		let nodes = txn.root_db(DB_NODES).ok_or_else(invalid_data_error)?;
		let sessions = txn.root_db(DB_SESSIONS).ok_or_else(invalid_data_error)?;
		let rsessions = txn.root_db(DB_RSESSIONS).ok_or_else(invalid_data_error)?;
		Ok(StoreReader { txn, id, links, rlinks, nodes, sessions, rsessions })
	}

	pub fn writer(&self) -> Result<StoreWriter, Error> {
		let txn = Env::mut_txn_begin(&self.env)?;
		let id = txn.root(ID_SQ).ok_or_else(invalid_data_error)?;
		let links = txn.root_db(DB_LINKS).ok_or_else(invalid_data_error)?;
		let rlinks = txn.root_db(DB_RLINKS).ok_or_else(invalid_data_error)?;
		let nodes = txn.root_db(DB_NODES).ok_or_else(invalid_data_error)?;
		let sessions = txn.root_db(DB_SESSIONS).ok_or_else(invalid_data_error)?;
		let rsessions = txn.root_db(DB_RSESSIONS).ok_or_else(invalid_data_error)?;
		Ok(StoreWriter { txn, id, links, rlinks, nodes, sessions, rsessions })
	}

	fn create_base(&self, root_data: &[u8]) -> Result<(), Error> {
		let mut txn = Env::mut_txn_begin(&self.env)?;

		let id = txn.root(ID_SQ);
		let links: Option<LinksDb> = txn.root_db(DB_LINKS);
		let rlinks: Option<RLinksDb> = txn.root_db(DB_RLINKS);
		let nodes: Option<NodesDb> = txn.root_db(DB_NODES);
		let sessions: Option<SessionsDb> = txn.root_db(DB_SESSIONS);
		let rsessions: Option<RSessionsDb> = txn.root_db(DB_RSESSIONS);
		match (id, links, rlinks, nodes, sessions, rsessions) {
			(Some(_), Some(_), Some(_), Some(nodes), Some(_), Some(_)) => {
				match btree::get(&txn, &nodes, &0, None)? {
					Some((&0, _)) => Ok(()),
					_ => Err(invalid_data_error()),
				}
			}
			(None, None, None, None, None, None) => {
				let links: LinksDb = btree::create_db(&mut txn)?;
				let rlinks: RLinksDb = btree::create_db(&mut txn)?;
				let mut nodes: NodesDb = btree::create_db_(&mut txn)?;
				let sessions: SessionsDb = btree::create_db(&mut txn)?;
				let rsessions: RSessionsDb = btree::create_db(&mut txn)?;

				btree::put(&mut txn, &mut nodes, &0, root_data)?;

				txn.set_root(ID_SQ, 1);
				txn.set_root(DB_LINKS, links.db);
				txn.set_root(DB_RLINKS, rlinks.db);
				txn.set_root(DB_NODES, nodes.db);
				txn.set_root(DB_SESSIONS, sessions.db);
				txn.set_root(DB_RSESSIONS, rsessions.db);
				txn.commit()
			}
			_ => {
				Err(invalid_data_error())
			}
		}
	}
}

pub type StoreReader<'env> = StoreRw<Txn<&'env Env>>;

pub struct StoreRw<T: LoadPage> {
	txn: T,
	id: u64,
	links: LinksDb,
	rlinks: RLinksDb,
	nodes: NodesDb,
	sessions: SessionsDb,
	rsessions: RSessionsDb,
}

impl<T: LoadPage<Error = Error>> StoreRw<T> {
	pub fn read(&self, id: u64) -> Result<Option<&[u8]>, Error> {
		match btree::get(&self.txn, &self.nodes, &id, None)? {
			Some((&eid, data)) if eid == id => Ok(Some(data)),
			_ => Ok(None)
		}
	}

	pub fn read_session(&self, id: u64) -> Result<Option<&Session>, Error> {
		match btree::get(&self.txn, &self.sessions, &id, None)? {
			Some((&eid, session)) if eid == id => Ok(Some(session)),
			_ => Ok(None)
		}
	}

	pub fn children<'s>(&'s self, id: u64) -> Result<impl Iterator<Item = Result<(u64, &'s[u8]), Error>>, Error> {
		Ok(self.child_ids(id)?.map(|child_id| match btree::get(&self.txn, &self.nodes, &child_id?, None)? {
			Some((&child_id, data)) => Ok((child_id, data)),
			None => Err(invalid_data_error()),
		}))
	}

	pub fn sessions<'s>(&'s self, id: u64) -> Result<impl Iterator<Item = Result<(&'s u64, &'s Session), Error>>, Error> {
		let iter = btree::iter(&self.txn, &self.sessions, Some((&id, None)))?;
		Ok(iter.take_while(move |entry| match entry {
			Ok((&eid, _)) if eid > id => false,
			_ => true,
		}))
	}

	pub fn all_sessions<'s>(&'s self) -> Result<impl Iterator<Item = Result<(&'s Session, &'s u64), Error>>, Error> {
		btree::iter(&self.txn, &self.rsessions, None)
	}

	fn child_ids(&self, id: u64) -> Result<ChildIdIter<'_, T>, Error> {
		Ok(ChildIdIter {
			reader: self,
			child_ids: ChildIds::new(self, id)?,
		})
	}

	fn get_child(&self, id: u64) -> Result<Option<u64>, Error> {
		match btree::get(&self.txn, &self.links, &id, None)? {
			Some((&eid, &child)) if eid == id => Ok(Some(child)),
			_ => Ok(None)
		}
	}

	fn get_rt(&self, id: u64, pid: u64) -> Result<Option<RTriple>, Error> {
		match btree::get(&self.txn, &self.rlinks, &id, Some(&RTriple { pid, next: 0, prev: 0 }))? {
			Some((&eid, &rt)) if eid == id && rt.pid == pid => Ok(Some(rt)),
			_ => Ok(None)
		}
	}
}

pub struct ChildIdIter<'reader, T: LoadPage<Error = Error>> {
	reader: &'reader StoreRw<T>,
	child_ids: ChildIds,
}

impl<'reader, T: LoadPage<Error = Error>> Iterator for ChildIdIter<'reader, T> {
	type Item = Result<u64, Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.child_ids.next(self.reader).transpose()
	}
}

struct ChildIds {
	pid: u64,
	id: u64,
}

impl ChildIds {
	fn new<T: LoadPage<Error = Error>>(reader: &StoreRw<T>, id: u64) -> Result<Self, Error> {
		Ok(ChildIds { pid: id, id: reader.get_child(id)?.unwrap_or(0) })
	}

	fn next<T: LoadPage<Error = Error>>(&mut self, reader: &StoreRw<T>) -> Result<Option<u64>, Error> {
		let id = self.id;
		if id == 0 { return Ok(None) };
		let RTriple { next, .. } = reader.get_rt(id, self.pid)?.ok_or_else(invalid_data_error)?;
		self.id = next;
		Ok(Some(id))
	}
}

type StoreWriter<'env> = StoreRw<MutTxn<&'env Env, ()>>;

impl<'env> StoreWriter<'env> {
	pub fn add_child(&mut self, pid: u64, data: &[u8]) -> Result<u64, Error> {
		let next = self.get_child(pid)?.unwrap_or(0);

		let id = self.id;
		btree::del(&mut self.txn, &mut self.links, &pid, None)?;
		btree::put(&mut self.txn, &mut self.links, &pid, &id)?;

		btree::put(&mut self.txn, &mut self.rlinks, &id, &RTriple { pid, next, prev: 0 })?;
		if next > 0 { self.modify_rt(next, pid, |rt| rt.prev = id)? };

		btree::put(&mut self.txn, &mut self.nodes, &id, data)?;
		self.id += 1;
		Ok(id)
	}

	pub fn add_session(&mut self, id: u64, session: &Session) -> Result<(), Error> {
		btree::put(&mut self.txn, &mut self.sessions, &id, session)?;
		btree::put(&mut self.txn, &mut self.rsessions, session, &id)?;
		Ok(())
	}

	pub fn delete(&mut self, pid: u64, id: u64) -> Result<(), Error> {
		btree::del(&mut self.txn, &mut self.links, &pid, Some(&id))?;

		let rt = self.get_rt(id, pid)?.ok_or_else(invalid_data_error)?;
		if rt.prev > 0 { self.modify_rt(rt.prev, pid, |prt| prt.next = rt.next)?; }
		else if rt.next > 0 { btree::put(&mut self.txn, &mut self.links, &pid, &rt.next)?; }
		if rt.next > 0 { self.modify_rt(rt.next, pid, |nrt| nrt.prev = rt.prev)? };

		self.delete_helper(pid, id)
	}

	pub fn modify(&mut self, id: u64, data: &[u8]) -> Result<(), Error> {
		btree::del(&mut self.txn, &mut self.nodes, &id, None)?;
		btree::put(&mut self.txn, &mut self.nodes, &id, data)?;
		Ok(())
	}

	pub fn move_up(&mut self, pid: u64, id: u64) -> Result<(), Error> {
		let rt = self.get_rt(id, pid)?.ok_or_else(invalid_data_error)?;
		let prt = if rt.prev > 0 {
			self.get_rt(rt.prev, pid)?.ok_or_else(invalid_data_error)?
		} else { return Ok(()) };

		self.modify_rt(id, pid, |crt| {
			crt.next = rt.prev;
			crt.prev = prt.prev;
		})?;

		if rt.next > 0 {
			self.modify_rt(rt.next, pid, |nrt| nrt.prev = rt.prev)?;
		}

		self.modify_rt(rt.prev, pid, |prt| {
			prt.next = rt.next;
			prt.prev = id;
		})?;

		if prt.prev > 0 {
			self.modify_rt(prt.prev, pid, |pprt| pprt.next = id)?;
		} else {
			btree::del(&mut self.txn, &mut self.links, &pid, None)?;
			btree::put(&mut self.txn, &mut self.links, &pid, &id)?;
		}

		Ok(())
	}

	pub fn move_down(&mut self, pid: u64, id: u64) -> Result<(), Error> {
		let rt = self.get_rt(id, pid)?.ok_or_else(invalid_data_error)?;
		let nrt = if rt.next > 0 {
			self.get_rt(rt.next, pid)?.ok_or_else(invalid_data_error)?
		} else { return Ok(()) };

		self.modify_rt(id, pid, |crt| {
			crt.prev = rt.next;
			crt.next = nrt.next;
		})?;

		if rt.prev > 0 {
			self.modify_rt(rt.prev, pid, |prt| prt.next = rt.next)?;
		} else {
			btree::del(&mut self.txn, &mut self.links, &pid, None)?;
			btree::put(&mut self.txn, &mut self.links, &pid, &rt.next)?;
		}

		self.modify_rt(rt.next, pid, |nrt| {
			nrt.prev = rt.prev;
			nrt.next = id;
		})?;

		if nrt.next > 0 {
			self.modify_rt(nrt.next, pid, |nnrt| nnrt.prev = id)?;
		}
		Ok(())
	}

	pub fn share(&mut self, src: u64, dest: u64) -> Result<bool, Error> {
		if self.is_descendent_of(dest, src)? { return Ok(false) };
		if self.get_rt(src, dest)?.is_some() { return Ok(false) };

		let next = self.get_child(dest)?.unwrap_or(0);

		btree::del(&mut self.txn, &mut self.links, &dest, None)?;
		btree::put(&mut self.txn, &mut self.links, &dest, &src)?;

		btree::put(&mut self.txn, &mut self.rlinks, &src, &RTriple { pid: dest, next, prev: 0 })?;
		if next > 0 { self.modify_rt(next, dest, |rt| rt.prev = src)? };

		Ok(true)
	}

	pub fn cut(&mut self, src_pid: u64, src: u64, dest: u64) -> Result<bool, Error> {
		if !self.share(src, dest)? { return Ok(false) };

		btree::del(&mut self.txn, &mut self.links, &src_pid, Some(&src))?;

		let rt = self.get_rt(src, src_pid)?.ok_or_else(invalid_data_error)?;
		if rt.prev > 0 { self.modify_rt(rt.prev, src_pid, |prt| prt.next = rt.next)? };
		if rt.next > 0 { self.modify_rt(rt.next, src_pid, |nrt| nrt.prev = rt.prev)? };
		btree::del(&mut self.txn, &mut self.rlinks, &src, Some(&rt))?;

		Ok(true)
	}

	pub fn commit(mut self) -> Result<(), Error> {
		self.txn.set_root(ID_SQ, self.id);
		self.txn.set_root(DB_LINKS, self.links.db);
		self.txn.set_root(DB_RLINKS, self.rlinks.db);
		self.txn.set_root(DB_NODES, self.nodes.db);
		self.txn.set_root(DB_SESSIONS, self.sessions.db);
		self.txn.set_root(DB_RSESSIONS, self.rsessions.db);
		self.txn.commit()
	}

	fn modify_rt(&mut self, id: u64, pid: u64, f: impl Fn(&mut RTriple)) -> Result<(), Error> {
		let mut rt = self.get_rt(id, pid)?.ok_or_else(invalid_data_error)?;
		btree::del(&mut self.txn, &mut self.rlinks, &id, Some(&rt))?;
		f(&mut rt);
		btree::put(&mut self.txn, &mut self.rlinks, &id, &rt)?;
		Ok(())
	}

	fn delete_helper(&mut self, pid: u64, id: u64) -> Result<(), Error> {
		let rt = self.get_rt(id, pid)?.ok_or_else(invalid_data_error)?;
		btree::del(&mut self.txn, &mut self.rlinks, &id, Some(&rt))?;

		if let Some((&eid, _)) = btree::get(&self.txn, &self.rlinks, &id, None)? {
			if eid == id { return Ok(()) };
		}

		btree::del(&mut self.txn, &mut self.nodes, &id, None)?;

		let mut child_ids = ChildIds::new(self, id)?;
		if let Some(first) = child_ids.next(self)? {
			btree::del(&mut self.txn, &mut self.links, &id, Some(&first))?;
			self.delete(id, first)?;
		}
		while let Some(child_id) = child_ids.next(self)? {
			self.delete(id, child_id)?;
		}
		Ok(())
	}

	fn is_descendent_of(&self, subj: u64, pred: u64) -> Result<bool, Error> {
		if subj == pred { return Ok(true) };
		for child_id in self.child_ids(pred)? {
			if self.is_descendent_of(subj, child_id?)? { return Ok(true) };
		}
		Ok(false)
	}
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
struct RTriple {
	pid: u64,
	next: u64,
	prev: u64,
}

impl Storable for RTriple {
	type PageReferences = core::iter::Empty<u64>;
	fn page_references(&self) -> Self::PageReferences {
		core::iter::empty()
	}

	fn compare<T>(&self, _: &T, b: &Self) -> core::cmp::Ordering {
		self.cmp(b)
	}
}

impl Storable for Session {
	type PageReferences = core::iter::Empty<u64>;
	fn page_references(&self) -> Self::PageReferences {
		core::iter::empty()
	}

	fn compare<T>(&self, _: &T, b: &Self) -> core::cmp::Ordering {
		self.cmp(b)
	}
}

fn invalid_data_error() -> Error {
	Error::IO(IoError::new(
		ErrorKind::InvalidData,
		"Database is invalid or corrupted"
	))
}

#[cfg(test)]
mod tests {
	use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
	use sanakirja::{Env, Error};
	use crate::node::Session;
	use super::Store;

	#[test]
	fn add_child() -> Result<(), Error> {
		let store = Store { env: Env::new_anon(1 << 14, 2)? };
		store.create_base(&[0])?;

		let mut writer = store.writer()?;
		writer.add_child(0, &[10, 20])?;
		writer.add_child(1, &[30, 20])?;
		writer.commit()?;

		let reader = store.reader()?;
		let mut iter = reader.children(0)?;

		let (id, data) = iter.next().unwrap()?;
		assert_eq!(id, 1);
		assert_eq!(data, &[10, 20]);
		assert!(iter.next().is_none());

		let mut iter = reader.children(1)?;
		let (id, data) = iter.next().unwrap()?;
		assert_eq!(id, 2);
		assert_eq!(data, &[30, 20]);
		assert!(iter.next().is_none());

		Ok(())
	}

	#[test]
	fn add_session() -> Result<(), Error> {
		let store = Store { env: Env::new_anon(1 << 14, 2)? };
		store.create_base(&[0])?;

		let mut writer = store.writer()?;
		let start = NaiveDateTime::new(
			NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
			NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
		);
		let end = NaiveDateTime::new(
			NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
			NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
		);
		let session = Session { start, end };
		writer.add_session(0, &session)?;
		writer.commit()?;

		let reader = store.reader()?;
		assert_eq!(reader.read_session(0)?.unwrap(), &session);

		Ok(())
	}
}
