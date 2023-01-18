use std::io::{Error as IoError, ErrorKind};
use std::path::Path;
use sanakirja::{Commit, Env, Error, MutTxn, RootDb, Storable, Txn};
use sanakirja::btree::{self, Db, UDb};

type LinksDb = Db<u64, u64>;
type RLinksDb = Db<u64, RPair>;
type NodesDb = UDb<u64, [u8]>;

const ID_SQ: usize = 0;
const DB_LINKS: usize = 1;
const DB_RLINKS: usize = 2;
const DB_NODES: usize = 3;

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
		let links = txn.root_db(DB_LINKS).ok_or_else(invalid_data_error)?;
		let nodes = txn.root_db(DB_NODES).ok_or_else(invalid_data_error)?;
		Ok(StoreReader { txn, links, nodes })
	}

	pub fn writer(&self) -> Result<StoreWriter, Error> {
		let txn = Env::mut_txn_begin(&self.env)?;
		let id = txn.root(ID_SQ).ok_or_else(invalid_data_error)?;
		let links = txn.root_db(DB_LINKS).ok_or_else(invalid_data_error)?;
		let rlinks = txn.root_db(DB_RLINKS).ok_or_else(invalid_data_error)?;
		let nodes = txn.root_db(DB_NODES).ok_or_else(invalid_data_error)?;
		Ok(StoreWriter { txn, id, links, rlinks, nodes })
	}

	fn create_base(&self, root_data: &[u8]) -> Result<(), Error> {
		let mut txn = Env::mut_txn_begin(&self.env)?;

		let id = txn.root(ID_SQ);
		let links: Option<LinksDb> = txn.root_db(DB_LINKS);
		let rlinks: Option<RLinksDb> = txn.root_db(DB_RLINKS);
		let nodes: Option<NodesDb> = txn.root_db(DB_NODES);
		match (id, links, rlinks, nodes) {
			(Some(_), Some(_), Some(_), Some(nodes)) => {
				match btree::get(&txn, &nodes, &0, None)? {
					Some((&0, _)) => Ok(()),
					_ => Err(invalid_data_error()),
				}
			}
			(None, None, None, None) => {
				let links: LinksDb = btree::create_db(&mut txn)?;
				let rlinks: RLinksDb = btree::create_db(&mut txn)?;
				let mut nodes: NodesDb = btree::create_db_(&mut txn)?;

				btree::put(&mut txn, &mut nodes, &0, root_data)?;

				txn.set_root(ID_SQ, 1);
				txn.set_root(DB_LINKS, links.db);
				txn.set_root(DB_RLINKS, rlinks.db);
				txn.set_root(DB_NODES, nodes.db);
				txn.commit()
			}
			_ => {
				Err(invalid_data_error())
			}
		}
	}
}

pub struct StoreReader<'env> {
	txn: Txn<&'env Env>,
	links: LinksDb,
	nodes: NodesDb,
}

impl<'env> StoreReader<'env> {
	pub fn read(&self, id: u64) -> Result<Option<&[u8]>, Error> {
		if let Some((&eid, data)) = btree::get(&self.txn, &self.nodes, &id, None)? {
			if eid == id { Ok(Some(data)) } else { Ok(None) }
		} else {
			Ok(None)
		}
	}

	pub fn children(&self, id: u64) -> Result<ChildIter, Error> {
		Ok(ChildIter {
			reader: self,
			child_ids: btree::iter(&self.txn, &self.links, Some((&id, None)))?,
			pid: id,
		})
	}
}

pub struct ChildIter<'env, 'reader> {
	reader: &'reader StoreReader<'env>,
	child_ids: btree::Iter<'reader, Txn<&'env Env>, u64, u64, btree::page::Page<u64, u64>>,
	pid: u64,
}

impl<'env, 'reader> Iterator for ChildIter<'env, 'reader> {
	type Item = Result<(u64, &'reader [u8]), Error>;

	fn next(&mut self) -> Option<Self::Item> {
		let child_id = match self.child_ids.next()? {
			Ok((&pid, child_id)) => {
				if pid != self.pid { return None }
				child_id
			}
			Err(err) => return Some(Err(err)),
		};
		match btree::get(&self.reader.txn, &self.reader.nodes, child_id, None) {
			Ok(Some((&id, data))) => Some(Ok((id, data))),
			Ok(None) => Some(Err(invalid_data_error())),
			Err(err) => Some(Err(err)),
		}
	}
}

pub struct StoreWriter<'env> {
	txn: MutTxn<&'env Env, ()>,
	id: u64,
	links: LinksDb,
	rlinks: RLinksDb,
	nodes: NodesDb,
}

impl<'env> StoreWriter<'env> {
	pub fn add_child(&mut self, pid: u64, data: &[u8]) -> Result<u64, Error> {
		let id = self.id;
		btree::put(&mut self.txn, &mut self.links, &pid, &id)?;
		btree::put(&mut self.txn, &mut self.rlinks, &id, &RPair { pid, prio: id })?;
		btree::put(&mut self.txn, &mut self.nodes, &id, data)?;
		self.id += 1;
		Ok(id)
	}

	pub fn delete(&mut self, pid: u64, id: u64) -> Result<(), Error> {
		if !btree::del(&mut self.txn, &mut self.links, &pid, Some(&id))? { return Ok(()) };

		let (&eid, &rpair) = btree::get(&self.txn, &self.rlinks, &id, Some(&RPair { pid, prio: 0 }))?
			.ok_or_else(invalid_data_error)?;
		if eid != id || pid != rpair.pid { return Err(invalid_data_error()) };
		btree::del(&mut self.txn, &mut self.rlinks, &id, Some(&rpair))?;

		if let Some((&eid, _)) = btree::get(&self.txn, &self.rlinks, &id, None)? {
			if eid == id { return Ok(()) };
		}

		btree::del(&mut self.txn, &mut self.nodes, &id, None)?;

		while let Some((&eid, &child_id)) = btree::get(&self.txn, &self.links, &id, None)? {
			if eid == id {
				self.delete(id, child_id)?;
			} else {
				break;
			}
		}
		Ok(())
	}

	pub fn modify(&mut self, id: u64, data: &[u8]) -> Result<(), Error> {
		btree::del(&mut self.txn, &mut self.nodes, &id, None)?;
		btree::put(&mut self.txn, &mut self.nodes, &id, data)?;
		Ok(())
	}

	pub fn share(&mut self, src: u64, dest: u64) -> Result<bool, Error> {
		if self.is_descendent_of(dest, src)? { return Ok(false) };
		if let Some((&pid, &id)) = btree::get(&self.txn, &self.links, &dest, Some(&src))? {
			if pid == dest && id == src { return Ok(false) };
		}

		btree::put(&mut self.txn, &mut self.links, &dest, &src)?;
		btree::put(&mut self.txn, &mut self.rlinks, &src, &RPair { pid: dest, prio: self.id })?;
		self.id += 1;

		Ok(true)
	}

	pub fn cut(&mut self, src_pid: u64, src: u64, dest: u64) -> Result<bool, Error> {
		if !self.share(src, dest)? { return Ok(false) };

		btree::del(&mut self.txn, &mut self.links, &src_pid, Some(&src))?;

		let (&eid, &rpair) = btree::get(&self.txn, &self.rlinks, &src, Some(&RPair { pid: src_pid, prio: 0 }))?
			.ok_or_else(invalid_data_error)?;
		if eid != src || src_pid != rpair.pid { return Err(invalid_data_error()) };
		btree::del(&mut self.txn, &mut self.rlinks, &src, Some(&rpair))?;

		Ok(true)
	}

	pub fn commit(mut self) -> Result<(), Error> {
		self.txn.set_root(ID_SQ, self.id);
		self.txn.set_root(DB_LINKS, self.links.db);
		self.txn.set_root(DB_RLINKS, self.rlinks.db);
		self.txn.set_root(DB_NODES, self.nodes.db);
		self.txn.commit()
	}

	fn is_descendent_of(&self, subj: u64, pred: u64) -> Result<bool, Error> {
		if subj == pred { return Ok(true) };
		for entry in btree::iter(&self.txn, &self.links, Some((&pred, None)))? {
			let (&id, &child_id) = entry?;
			if id != pred { break };
			if self.is_descendent_of(subj, child_id)? { return Ok(true) };
		}
		Ok(false)
	}
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
struct RPair {
	pid: u64,
	prio: u64,
}

impl Storable for RPair {
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
	use sanakirja::{Env, Error};
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
}
