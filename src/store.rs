use std::io::{Error as IoError, ErrorKind};
use std::path::Path;
use sanakirja::{Commit, Env, Error, MutTxn, RootDb, Txn};
use sanakirja::btree::{self, Db, UDb};

const ID_SQ: usize = 0;
const DB_LINKS: usize = 1;
const DB_NODES: usize = 2;

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
		let links: Db<u64, u64> = txn.root_db(DB_LINKS).ok_or_else(invalid_data_error)?;
		let nodes: UDb<u64, [u8]> = txn.root_db(DB_NODES).ok_or_else(invalid_data_error)?;
		Ok(StoreReader { txn, links, nodes })
	}

	pub fn writer(&self) -> Result<StoreWriter, Error> {
		let txn = Env::mut_txn_begin(&self.env)?;
		let id = txn.root(ID_SQ).ok_or_else(invalid_data_error)?;
		let links: Db<u64, u64> = txn.root_db(DB_LINKS).ok_or_else(invalid_data_error)?;
		let nodes: UDb<u64, [u8]> = txn.root_db(DB_NODES).ok_or_else(invalid_data_error)?;
		Ok(StoreWriter { txn, id, links, nodes })
	}

	fn create_base(&self, root_data: &[u8]) -> Result<(), Error> {
		let mut txn = Env::mut_txn_begin(&self.env)?;

		let id = txn.root(ID_SQ);
		let links: Option<Db<u64, u64>> = txn.root_db(DB_LINKS);
		let nodes: Option<UDb<u64, [u8]>> = txn.root_db(DB_NODES);
		match (id, links, nodes) {
			(Some(_), Some(_), Some(nodes)) => {
				match btree::get(&txn, &nodes, &0, None)? {
					Some((&0, _)) => Ok(()),
					_ => Err(invalid_data_error()),
				}
			}
			(None, None, None) => {
				let links: Db<u64, u64> = btree::create_db(&mut txn)?;
				let mut nodes: UDb<u64, [u8]> = btree::create_db_(&mut txn)?;

				btree::put(&mut txn, &mut nodes, &0, root_data)?;

				txn.set_root(ID_SQ, 1);
				txn.set_root(DB_LINKS, links.db);
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
	links: Db<u64, u64>,
	nodes: UDb<u64, [u8]>,
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
	links: Db<u64, u64>,
	nodes: UDb<u64, [u8]>,
}

impl<'env> StoreWriter<'env> {
	pub fn add_child(&mut self, pid: u64, data: &[u8]) -> Result<(), Error> {
		btree::put(&mut self.txn, &mut self.links, &pid, &self.id)?;
		btree::put(&mut self.txn, &mut self.nodes, &self.id, data)?;
		self.id += 1;
		Ok(())
	}

	pub fn delete(&mut self, pid: u64, id: u64) -> Result<(), Error> {
		while let Some((_, &child_id)) = btree::get(&self.txn, &self.links, &id, None)? {
			self.delete(id, child_id)?;
		}
		btree::del(&mut self.txn, &mut self.nodes, &id, None)?;
		btree::del(&mut self.txn, &mut self.links, &pid, Some(&id))?;
		Ok(())
	}

	pub fn modify(&mut self, id: u64, data: &[u8]) -> Result<(), Error> {
		btree::del(&mut self.txn, &mut self.nodes, &id, None)?;
		btree::put(&mut self.txn, &mut self.nodes, &id, data)?;
		Ok(())
	}

	pub fn commit(mut self) -> Result<(), Error> {
		self.txn.set_root(ID_SQ, self.id);
		self.txn.set_root(DB_LINKS, self.links.db);
		self.txn.set_root(DB_NODES, self.nodes.db);
		self.txn.commit()
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
