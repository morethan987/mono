use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::models::Task;
use crate::scheduling::policy::ScoredTask;

struct HeapEntry {
    scored_task: ScoredTask,
}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.scored_task.score == other.scored_task.score
    }
}

impl Eq for HeapEntry {}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.scored_task
            .score
            .partial_cmp(&other.scored_task.score)
            .unwrap_or(Ordering::Equal)
    }
}

pub struct TaskQueue {
    heap: BinaryHeap<HeapEntry>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            heap: BinaryHeap::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, scored_task: ScoredTask) {
        self.heap.push(HeapEntry { scored_task });
    }

    pub fn pop(&mut self) -> Option<ScoredTask> {
        self.heap.pop().map(|entry| entry.scored_task)
    }

    pub fn peek(&self) -> Option<&ScoredTask> {
        self.heap.peek().map(|entry| &entry.scored_task)
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn clear(&mut self) {
        self.heap.clear();
    }

    pub fn into_sorted_vec(self) -> Vec<ScoredTask> {
        self.heap
            .into_sorted_vec()
            .into_iter()
            .rev()
            .map(|entry| entry.scored_task)
            .collect()
    }

    pub fn from_tasks(tasks: impl IntoIterator<Item = ScoredTask>) -> Self {
        let mut queue = Self::new();
        for task in tasks {
            queue.push(task);
        }
        queue
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<ScoredTask> for TaskQueue {
    fn from_iter<I: IntoIterator<Item = ScoredTask>>(iter: I) -> Self {
        Self::from_tasks(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Task;

    #[test]
    fn test_priority_queue_ordering() {
        let mut queue = TaskQueue::new();

        queue.push(ScoredTask::new(Task::new("Low".to_string()), 0.2));
        queue.push(ScoredTask::new(Task::new("High".to_string()), 0.9));
        queue.push(ScoredTask::new(Task::new("Medium".to_string()), 0.5));

        assert_eq!(queue.pop().unwrap().task.title, "High");
        assert_eq!(queue.pop().unwrap().task.title, "Medium");
        assert_eq!(queue.pop().unwrap().task.title, "Low");
    }

    #[test]
    fn test_peek() {
        let mut queue = TaskQueue::new();
        queue.push(ScoredTask::new(Task::new("Task".to_string()), 0.5));

        assert_eq!(queue.peek().unwrap().task.title, "Task");
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_into_sorted_vec() {
        let mut queue = TaskQueue::new();
        queue.push(ScoredTask::new(Task::new("Low".to_string()), 0.2));
        queue.push(ScoredTask::new(Task::new("High".to_string()), 0.9));
        queue.push(ScoredTask::new(Task::new("Medium".to_string()), 0.5));

        let sorted = queue.into_sorted_vec();
        assert_eq!(sorted[0].task.title, "High");
        assert_eq!(sorted[1].task.title, "Medium");
        assert_eq!(sorted[2].task.title, "Low");
    }
}
