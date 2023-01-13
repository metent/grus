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
		let filled = root.splits.len() - 1;
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
		let extra = child.splits.len() - 1;
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
		children.sort_by(|l, r| l.data.priority.cmp(&r.data.priority));
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
	use crate::node::{Node, NodeData, Priority};
	use super::{FlatTreeBuilder, FlatTreeState};

	#[test]
	fn build_flattree() {
		let nodes = [
			Node {
				id: 0,
				pid: 0,
				depth: 0,
				data: NodeData {
					name: "/".into(),
					priority: Priority::None,
					..NodeData::default()
				},
				splits: vec![0, 1],
			},
			Node {
				id: 1,
				pid: 0,
				depth: 1,
				data: NodeData {
					name: "a".into(),
					priority: Priority::High,
					..NodeData::default()
				},
				splits: vec![0, 1],
			},
			Node {
				id: 4,
				pid: 1,
				depth: 2,
				data: NodeData {
					name: "x".into(),
					priority: Priority::High,
					..NodeData::default()
				},
				splits: vec![0, 1],
			},
			Node {
				id: 5,
				pid: 1,
				depth: 2,
				data: NodeData {
					name: "y".into(),
					priority: Priority::Medium,
					..NodeData::default()
				},
				splits: vec![0, 1],
			},
			Node {
				id: 3,
				pid: 0,
				depth: 1,
				data: NodeData {
					name: "b".into(),
					priority: Priority::Medium,
					..NodeData::default()
				},
				splits: vec![0, 1],
			},
			Node {
				id: 2,
				pid: 0,
				depth: 1,
				data: NodeData {
					name: "c".into(),
					priority: Priority::Low,
					..NodeData::default()
				},
				splits: vec![0, 1],
			},
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

}
