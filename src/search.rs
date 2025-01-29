use crate::{GraphMappings, StepId, ValueId};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NodeId {
    Step(StepId),
    Value(ValueId),
}

pub trait QueueContainer<T> {
    fn new(init: T) -> Self;

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>;
    fn pop(&mut self) -> Option<T>;
}

pub struct BfsQueue<T> {
    queue: VecDeque<T>,
    step: usize,
}

impl<T> BfsQueue<T> {
    pub fn current_step(&self) -> usize {
        self.step
    }
}

impl<T> QueueContainer<T> for BfsQueue<T> {
    fn new(init: T) -> Self {
        Self {
            queue: VecDeque::from([init]),
            step: 0,
        }
    }

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.queue.extend(iter);
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.pop_front().inspect(|_| self.step += 1)
    }
}

pub struct DfsQueue<T> {
    queue: Vec<T>,
    step: usize,
}

impl<T> DfsQueue<T> {
    pub fn current_step(&self) -> usize {
        self.step
    }
}

impl<T> QueueContainer<T> for DfsQueue<T> {
    fn new(init: T) -> Self {
        Self {
            queue: Vec::from([init]),
            step: 0,
        }
    }

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.queue.extend(iter);
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.pop().inspect(|_| self.step += 1)
    }
}

pub struct SearchAlgorithmIter<'a, Q>
where
    Q: QueueContainer<Vec<NodeId>>,
{
    mappings: &'a GraphMappings,

    // visited: HashSet<NodeId>,
    queue: Q,

    target: ValueId,
}

impl<Q> SearchAlgorithmIter<'_, Q>
where
    Q: QueueContainer<Vec<NodeId>>,
{
    pub fn queue(&self) -> &Q {
        &self.queue
    }

    pub fn into_queue(self) -> Q {
        self.queue
    }
}

impl<Q> Iterator for SearchAlgorithmIter<'_, Q>
where
    Q: QueueContainer<Vec<NodeId>>,
{
    type Item = Vec<NodeId>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(path) = self.queue.pop() {
            if *path.last().unwrap() == NodeId::Value(self.target) {
                return Some(path);
            }

            match *path.last().unwrap() {
                NodeId::Step(id) => {
                    self.queue.extend(
                        self.mappings[id]
                            .iter()
                            .copied()
                            .filter(|x| {
                                // self.visited.insert(NodeId::Value(*x))
                                !path.contains(&NodeId::Value(*x))
                            })
                            .map(|x| {
                                let mut new_path = path.clone();
                                new_path.push(NodeId::Value(x));
                                new_path
                            }),
                    );
                }
                NodeId::Value(id) => {
                    self.queue.extend(
                        self.mappings[id]
                            .iter()
                            .copied()
                            .filter(|x| {
                                // self.visited.insert(NodeId::Step(*x))
                                !path.contains(&NodeId::Step(*x))
                            })
                            .map(|x| {
                                let mut new_path = path.clone();
                                new_path.push(NodeId::Step(x));
                                new_path
                            }),
                    );
                }
            }
        }

        None
    }
}

pub fn run_search_algorithm<Q>(
    mappings: &GraphMappings,
    source: ValueId,
    target: ValueId,
) -> SearchAlgorithmIter<Q>
where
    Q: QueueContainer<Vec<NodeId>>,
{
    SearchAlgorithmIter {
        mappings,
        // visited: HashSet::from([NodeId::Value(source)]),
        queue: Q::new(vec![NodeId::Value(source)]),
        target,
    }
}
