use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    sync::{
        atomic::{AtomicUsize, Ordering as AtomicOrdering},
        Mutex,
    },
};

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

use crate::{
    error::TaskError,
    task::{Priority, TaskId},
};

#[derive(Debug)]
pub struct SchedulerEntry {
    pub task_id: TaskId,
    pub priority: Priority,
    pub enqueued_at: DateTime<Utc>,
    pub cancellation: CancellationToken,
}

impl PartialEq for SchedulerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for SchedulerEntry {}

impl PartialOrd for SchedulerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SchedulerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => {
                // If priorities are equal, older tasks (smaller timestamp) have higher precedence
                other.enqueued_at.cmp(&self.enqueued_at)
            }
            other_order => other_order,
        }
    }
}

/// Priority-based task scheduler with depth limits.
pub struct Scheduler {
    queue: Mutex<BinaryHeap<SchedulerEntry>>,
    max_depth: usize,
    active_count: AtomicUsize,
}

impl Scheduler {
    pub fn new(max_depth: usize) -> Self {
        Self { queue: Mutex::new(BinaryHeap::new()), max_depth, active_count: AtomicUsize::new(0) }
    }

    /// Enqueue a task. Returns error if queue is full.
    pub async fn enqueue(
        &self,
        task_id: TaskId,
        priority: Priority,
    ) -> Result<CancellationToken, TaskError> {
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());

        if queue.len() >= self.max_depth {
            return Err(TaskError::QueueFull { max_depth: self.max_depth });
        }

        let token = CancellationToken::new();
        let entry = SchedulerEntry {
            task_id,
            priority,
            enqueued_at: Utc::now(),
            cancellation: token.clone(),
        };

        queue.push(entry);
        self.active_count.store(queue.len(), AtomicOrdering::SeqCst);

        Ok(token)
    }

    /// Dequeue the highest-priority task.
    pub async fn dequeue(&self) -> Option<SchedulerEntry> {
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        let entry = queue.pop()?;
        self.active_count.store(queue.len(), AtomicOrdering::SeqCst);
        Some(entry)
    }

    /// Get current queue depth.
    pub fn queue_depth(&self) -> usize {
        self.active_count.load(AtomicOrdering::SeqCst)
    }

    /// Cancel a specific task.
    pub async fn cancel(&self, task_id: &TaskId) -> bool {
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());

        // BinaryHeap doesn't support easy removal by identity, so we rebuild it
        let mut new_heap = BinaryHeap::new();
        let mut found = false;

        for entry in queue.drain() {
            if entry.task_id == *task_id {
                entry.cancellation.cancel();
                found = true;
            } else {
                new_heap.push(entry);
            }
        }

        *queue = new_heap;
        self.active_count.store(queue.len(), AtomicOrdering::SeqCst);
        found
    }

    /// Drain all entries (for shutdown).
    pub async fn drain(&self) -> Vec<SchedulerEntry> {
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        let drained: Vec<_> = queue.drain().collect();
        self.active_count.store(0, AtomicOrdering::SeqCst);
        drained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let scheduler = Scheduler::new(10);
        let task_id = TaskId::new();
        let _ = scheduler.enqueue(task_id, Priority::Normal).await.unwrap();
        assert_eq!(scheduler.queue_depth(), 1);

        let entry = scheduler.dequeue().await.unwrap();
        assert_eq!(entry.task_id, task_id);
        assert_eq!(scheduler.queue_depth(), 0);
    }

    #[tokio::test]
    async fn test_overflow() {
        let scheduler = Scheduler::new(1);
        let _ = scheduler.enqueue(TaskId::new(), Priority::Normal).await.unwrap();
        let result = scheduler.enqueue(TaskId::new(), Priority::Normal).await;
        assert!(matches!(result, Err(TaskError::QueueFull { .. })));
    }

    #[tokio::test]
    async fn test_cancel() {
        let scheduler = Scheduler::new(10);
        let task_id = TaskId::new();
        let _ = scheduler.enqueue(task_id, Priority::Normal).await.unwrap();
        assert!(scheduler.cancel(&task_id).await);
        assert_eq!(scheduler.queue_depth(), 0);
        assert!(scheduler.dequeue().await.is_none());
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let scheduler = Scheduler::new(10);
        let task1 = TaskId::new();
        let task2 = TaskId::new();
        let task3 = TaskId::new();

        let _ = scheduler.enqueue(task1, Priority::Low).await.unwrap();
        let _ = scheduler.enqueue(task2, Priority::High).await.unwrap();
        let _ = scheduler.enqueue(task3, Priority::Normal).await.unwrap();

        assert_eq!(scheduler.dequeue().await.unwrap().task_id, task2);
        assert_eq!(scheduler.dequeue().await.unwrap().task_id, task3);
        assert_eq!(scheduler.dequeue().await.unwrap().task_id, task1);
    }

    #[tokio::test]
    async fn test_dequeue_empty_queue() {
        let scheduler = Scheduler::new(10);
        let result = scheduler.dequeue().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_queue_depth_initial() {
        let scheduler = Scheduler::new(10);
        assert_eq!(scheduler.queue_depth(), 0);
    }

    #[tokio::test]
    async fn test_queue_depth_after_enqueue() {
        let scheduler = Scheduler::new(10);
        let _ = scheduler.enqueue(TaskId::new(), Priority::Normal).await.unwrap();
        assert_eq!(scheduler.queue_depth(), 1);
    }

    #[tokio::test]
    async fn test_queue_depth_after_dequeue() {
        let scheduler = Scheduler::new(10);
        let _ = scheduler.enqueue(TaskId::new(), Priority::Normal).await.unwrap();
        assert_eq!(scheduler.queue_depth(), 1);
        let _ = scheduler.dequeue().await.unwrap();
        assert_eq!(scheduler.queue_depth(), 0);
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_task() {
        let scheduler = Scheduler::new(10);
        let fake_id = TaskId::new();
        let result = scheduler.cancel(&fake_id).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_cancel_then_dequeue_none() {
        let scheduler = Scheduler::new(10);
        let task_id = TaskId::new();
        let _ = scheduler.enqueue(task_id, Priority::Normal).await.unwrap();
        assert!(scheduler.cancel(&task_id).await);
        assert_eq!(scheduler.queue_depth(), 0);
        assert!(scheduler.dequeue().await.is_none());
    }

    #[tokio::test]
    async fn test_drain() {
        let scheduler = Scheduler::new(10);
        let t1 = TaskId::new();
        let t2 = TaskId::new();
        let _ = scheduler.enqueue(t1, Priority::Normal).await.unwrap();
        let _ = scheduler.enqueue(t2, Priority::High).await.unwrap();
        assert_eq!(scheduler.queue_depth(), 2);

        let drained = scheduler.drain().await;
        assert_eq!(drained.len(), 2);
        assert_eq!(scheduler.queue_depth(), 0);
        assert!(scheduler.dequeue().await.is_none());
    }

    #[tokio::test]
    async fn test_drain_empty() {
        let scheduler = Scheduler::new(10);
        let drained = scheduler.drain().await;
        assert!(drained.is_empty());
    }

    #[tokio::test]
    async fn test_enqueue_dequeue_critical_priority() {
        let scheduler = Scheduler::new(10);
        let low_id = TaskId::new();
        let crit_id = TaskId::new();
        let _ = scheduler.enqueue(low_id, Priority::Low).await.unwrap();
        let _ = scheduler.enqueue(crit_id, Priority::Critical).await.unwrap();
        assert_eq!(scheduler.dequeue().await.unwrap().task_id, crit_id);
    }

    #[tokio::test]
    async fn test_enqueue_max_depth_zero() {
        let scheduler = Scheduler::new(0);
        let result = scheduler.enqueue(TaskId::new(), Priority::Normal).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scheduler_entry_ord_implementation() {
        use tokio_util::sync::CancellationToken;

        let t1 = SchedulerEntry {
            task_id: TaskId::new(),
            priority: Priority::High,
            enqueued_at: Utc::now(),
            cancellation: CancellationToken::new(),
        };
        let t2 = SchedulerEntry {
            task_id: TaskId::new(),
            priority: Priority::Low,
            enqueued_at: Utc::now(),
            cancellation: CancellationToken::new(),
        };
        // Higher priority should be greater in Ord (so it pops first from max-heap)
        assert!(t1 > t2);
    }

    #[tokio::test]
    async fn test_fifo_within_same_priority() {
        let scheduler = Scheduler::new(10);
        let t1 = TaskId::new();
        let t2 = TaskId::new();
        let _ = scheduler.enqueue(t1, Priority::Normal).await.unwrap();
        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let _ = scheduler.enqueue(t2, Priority::Normal).await.unwrap();

        // Earlier enqueued should come first
        assert_eq!(scheduler.dequeue().await.unwrap().task_id, t1);
        assert_eq!(scheduler.dequeue().await.unwrap().task_id, t2);
    }

    #[tokio::test]
    async fn test_enqueue_dequeue_mixed_priorities() {
        let scheduler = Scheduler::new(10);
        for _ in 0..5 {
            let _ = scheduler.enqueue(TaskId::new(), Priority::Low).await.unwrap();
        }
        let _ = scheduler.enqueue(TaskId::new(), Priority::Critical).await.unwrap();
        for _ in 0..3 {
            let _ = scheduler.enqueue(TaskId::new(), Priority::High).await.unwrap();
        }

        // Critical first, then High, then Low
        let first = scheduler.dequeue().await.unwrap();
        assert_eq!(first.priority, Priority::Critical);
        for _ in 0..3 {
            assert_eq!(scheduler.dequeue().await.unwrap().priority, Priority::High);
        }
        for _ in 0..5 {
            assert_eq!(scheduler.dequeue().await.unwrap().priority, Priority::Low);
        }
    }

    #[tokio::test]
    async fn test_cancel_middle_of_queue() {
        let scheduler = Scheduler::new(10);
        let t1 = TaskId::new();
        let t2 = TaskId::new();
        let t3 = TaskId::new();
        let _ = scheduler.enqueue(t1, Priority::Normal).await.unwrap();
        let _ = scheduler.enqueue(t2, Priority::Normal).await.unwrap();
        let _ = scheduler.enqueue(t3, Priority::Normal).await.unwrap();

        // Cancel the middle one
        assert!(scheduler.cancel(&t2).await);
        assert_eq!(scheduler.queue_depth(), 2);

        let first = scheduler.dequeue().await.unwrap();
        assert_eq!(first.task_id, t1);
        let second = scheduler.dequeue().await.unwrap();
        assert_eq!(second.task_id, t3);
    }
}
