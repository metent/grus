use std::collections::VecDeque;
use std::ops::Range;
use crate::node::Node;

pub struct FlatTreeBuilder {
	height: usize,
	fnodes: Vec<FNode>,
	queue: VecDeque<FChildIter>,
	start: usize,
	filled: usize,
}

impl FlatTreeBuilder {
	pub fn new(root: Node<'static>, height: usize) -> Self {
		let filled = root.height();
		let fnodes = vec![FNode { node: root, path: vec![0] }];
		FlatTreeBuilder { height, fnodes, queue: VecDeque::new(), start: 0, filled }
	}

	pub fn step(&mut self) -> FlatTreeState {
		let Some(mut children) = self.queue.pop_front() else {
			if self.start == self.fnodes.len() {
				return FlatTreeState::Done
			} else {
				return FlatTreeState::Refill
			}
		};

		let Some(child) = children.iter.next() else { return FlatTreeState::Build };
		let extra = child.height();
		if self.filled + extra > self.height { return FlatTreeState::Done }
		self.filled += extra;

		let mut path = self.fnodes[children.last].path.clone();
		path.push(self.fnodes.len());

		self.queue.push_back(children);
		self.fnodes.push(FNode { node: child, path });

		FlatTreeState::Build
	}

	pub fn fill_range(&self) -> Range<usize> {
		self.start..self.fnodes.len()
	}

	pub fn id(&self, i: usize) -> u64 {
		self.fnodes[i].node.id
	}

	pub fn depth(&self, i: usize) -> usize {
		self.fnodes[i].node.depth
	}

	pub fn fill(&mut self, mut children: Vec<Node<'static>>, last: usize) {
		children.sort_by(|l, r| l.priority.det.cmp(&r.priority.det));
		self.queue.push_back(FChildIter {
			iter: children.into_iter(),
			last
		});
	}

	pub fn finish_fill(&mut self) {
		self.start = self.fnodes.len();
	}

	pub fn finish(mut self) -> Vec<Node<'static>> {
		self.fnodes.sort_by(|l, r| l.path.cmp(&r.path));
		self.fnodes.into_iter().map(|fnode| fnode.node).collect()
	}
}

struct FNode {
	node: Node<'static>,
	path: Vec<usize>,
}

struct FChildIter {
	iter: std::vec::IntoIter<Node<'static>>,
	last: usize,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum FlatTreeState {
	Build,
	Refill,
	Done,
}

#[cfg(test)]
mod tests {
	use crate::node::{Node, NodeData, Priority, Session};
	use super::{FlatTreeBuilder, FlatTreeState};

	#[test]
	fn build_flattree() {
		let nodes = [
			create_node(0, 0, "/", Priority { det: 0, total: 1 }),
			create_node(0, 1, "a", Priority { det: 0, total: 3 }),
			create_node(1, 4, "x", Priority { det: 0, total: 2 }),
			create_node(1, 5, "y", Priority { det: 1, total: 2 }),
			create_node(0, 3, "b", Priority { det: 1, total: 3 }),
			create_node(0, 2, "c", Priority { det: 2, total: 3 }),
		];
		let first_level_nodes = vec![nodes[5].clone(), nodes[1].clone(), nodes[4].clone()];
		let a_children = vec![nodes[2].clone(), nodes[3].clone()];

		let mut builder = FlatTreeBuilder::new(nodes[0].clone(), 10);

		assert_eq!(builder.step(), FlatTreeState::Refill);
		assert_eq!(builder.fill_range(), 0..1);
		builder.fill(first_level_nodes, 0);
		builder.finish_fill();

		assert_eq!(builder.step(), FlatTreeState::Build);
		assert_eq!(builder.step(), FlatTreeState::Build);
		assert_eq!(builder.step(), FlatTreeState::Build);
		assert_eq!(builder.step(), FlatTreeState::Build);

		assert_eq!(builder.step(), FlatTreeState::Refill);
		assert_eq!(builder.fill_range(), 1..4);
		builder.fill(a_children, 1);
		builder.fill(Vec::new(), 2);
		builder.fill(Vec::new(), 3);
		builder.finish_fill();

		assert_eq!(builder.step(), FlatTreeState::Build);
		assert_eq!(builder.step(), FlatTreeState::Build);
		assert_eq!(builder.step(), FlatTreeState::Build);
		assert_eq!(builder.step(), FlatTreeState::Build);
		assert_eq!(builder.step(), FlatTreeState::Build);

		assert_eq!(builder.step(), FlatTreeState::Refill);
		assert_eq!(builder.fill_range(), 4..6);
		builder.fill(Vec::new(), 4);
		builder.fill(Vec::new(), 5);
		builder.finish_fill();

		assert_eq!(builder.step(), FlatTreeState::Build);
		assert_eq!(builder.step(), FlatTreeState::Build);

		assert_eq!(builder.step(), FlatTreeState::Done);
		assert_eq!(builder.finish(), nodes);
	}

	fn create_node(pid: u64, id: u64, name: &'static str, pri: Priority) -> Node<'static> {
		Node {
			id,
			pid,
			depth: 0,
			data: NodeData {
				name: name.into(),
				..NodeData::default()
			},
			session: Some(Session::default()),
			priority: pri,
			name_splits: vec![0, 1],
			session_text: "".into(),
			session_splits: vec![0, 0],
			due_date_text: "".into(),
			due_date_splits: vec![0, 0],
		}
	}
}
