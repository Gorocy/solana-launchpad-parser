use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::warn;

/// Structure representing a transaction in the queue
#[derive(Clone, Debug)]
pub struct QueuedTransaction {
    pub signature: String,
    pub slot: u64,
    pub received_time: DateTime<Utc>,
    pub accounts: Vec<String>,
    pub instructions: Vec<TransactionInstruction>,
}

/// Instruction data from transaction
#[derive(Clone, Debug)]
pub struct TransactionInstruction {
    pub program_id: String,
    pub accounts: Vec<u8>, // Account indices
    pub data: Vec<u8>,     // Instruction data
}

/// Thread-safe queue for transactions
#[derive(Clone)]
pub struct TransactionQueue {
    queue: Arc<Mutex<VecDeque<QueuedTransaction>>>,
    max_size: usize,
}

impl TransactionQueue {
    /// Creates a new queue with specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            max_size,
        }
    }

    /// Adds transaction to queue
    pub async fn push(&self, transaction: QueuedTransaction) {
        let mut queue = self.queue.lock().await;

        // Remove oldest transactions if exceeding limit
        while queue.len() >= self.max_size {
            if let Some(_removed) = queue.pop_front() {
                warn!("Removed oldest transaction from queue");
            }
        }

        queue.push_back(transaction);
    }

    /// Gets transaction from queue (FIFO)
    pub async fn pop(&self) -> Option<QueuedTransaction> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }

    /// Returns current queue size
    pub async fn len(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }

    /// Checks if queue is empty
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.lock().await;
        queue.is_empty()
    }

    /// Gets all transactions from queue (clears queue)
    pub async fn drain_all(&self) -> Vec<QueuedTransaction> {
        let mut queue = self.queue.lock().await;
        queue.drain(..).collect()
    }

    /// Gets a batch of transactions from queue (up to max_count)
    pub async fn pop_batch(&self, max_count: usize) -> Vec<QueuedTransaction> {
        let mut queue = self.queue.lock().await;
        let mut batch = Vec::new();
        
        for _ in 0..max_count {
            if let Some(transaction) = queue.pop_front() {
                batch.push(transaction);
            } else {
                break;
            }
        }
        
        batch
    }
}
