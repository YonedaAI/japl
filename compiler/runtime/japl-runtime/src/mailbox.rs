//! Lock-free MPSC mailbox with selective receive for JAPL processes.
//!
//! Each process has a single mailbox. Messages are sent by any process
//! (multi-producer) and consumed only by the owning process (single-consumer).
//! The mailbox supports:
//! - Standard FIFO receive
//! - Selective receive with a predicate (scan for matching messages)
//! - Timeout-based receive
//!
//! Non-matching messages during selective receive are saved in a secondary
//! queue and restored on the next receive operation.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossbeam_queue::SegQueue;

use crate::value::Value;

/// A process mailbox implementing lock-free MPSC semantics.
///
/// The `queue` field is the primary lock-free queue used for concurrent
/// message sends from any thread. The `save_queue` holds messages that
/// were skipped during selective receive, to be re-examined later.
pub struct Mailbox {
    /// Lock-free concurrent queue for incoming messages.
    /// Any process/thread can enqueue; only the owning process dequeues.
    queue: SegQueue<Value>,
    /// Messages skipped during selective receive.
    /// These are prepended back before the next receive.
    save_queue: VecDeque<Value>,
}

impl Mailbox {
    /// Create a new empty mailbox.
    pub fn new() -> Self {
        Mailbox {
            queue: SegQueue::new(),
            save_queue: VecDeque::new(),
        }
    }

    /// Send a message to this mailbox.
    ///
    /// This is thread-safe and lock-free. Can be called from any thread.
    pub fn send(&self, msg: Value) {
        self.queue.push(msg);
    }

    /// Receive the next message, checking saved messages first.
    ///
    /// Returns `None` if no messages are available (non-blocking).
    pub fn try_receive(&mut self) -> Option<Value> {
        // Check save queue first (messages from previous selective receives)
        if let Some(msg) = self.save_queue.pop_front() {
            return Some(msg);
        }
        // Then check the concurrent queue
        self.queue.pop()
    }

    /// Receive a message, blocking until one is available or timeout expires.
    ///
    /// If `timeout` is `None`, blocks indefinitely (busy-waits with yields).
    /// If `timeout` is `Some(duration)`, returns `None` after the duration.
    pub fn receive(&mut self, timeout: Option<Duration>) -> Option<Value> {
        // Try immediate receive first
        if let Some(msg) = self.try_receive() {
            return Some(msg);
        }

        let deadline = timeout.map(|d| Instant::now() + d);

        loop {
            if let Some(msg) = self.try_receive() {
                return Some(msg);
            }

            // Check timeout
            if let Some(deadline) = deadline {
                if Instant::now() >= deadline {
                    return None;
                }
            }

            // Yield to other threads while waiting
            std::thread::yield_now();
        }
    }

    /// Selective receive: find the first message matching the predicate.
    ///
    /// Scans the save queue and then the incoming queue for a message
    /// that satisfies `matcher`. Non-matching messages are saved in the
    /// save queue for later retrieval.
    ///
    /// This implements Erlang-style selective receive where messages can
    /// be consumed out of order based on pattern matching.
    pub fn selective_receive<F>(
        &mut self,
        matcher: F,
        timeout: Option<Duration>,
    ) -> Option<Value>
    where
        F: Fn(&Value) -> bool,
    {
        // First, scan the save queue for a match
        for i in 0..self.save_queue.len() {
            if matcher(&self.save_queue[i]) {
                return self.save_queue.remove(i);
            }
        }

        let deadline = timeout.map(|d| Instant::now() + d);

        // Scan incoming messages
        loop {
            match self.queue.pop() {
                Some(msg) => {
                    if matcher(&msg) {
                        return Some(msg);
                    }
                    // Not matching -- save for later
                    self.save_queue.push_back(msg);
                }
                None => {
                    // No more messages available
                    if let Some(deadline) = deadline {
                        if Instant::now() >= deadline {
                            return None;
                        }
                    }

                    // Brief yield before retrying
                    std::thread::yield_now();

                    // For non-blocking check, if no timeout and no messages, return None
                    if timeout.is_none() && self.queue.is_empty() {
                        // Check one more time after yield
                        if let Some(msg) = self.queue.pop() {
                            if matcher(&msg) {
                                return Some(msg);
                            }
                            self.save_queue.push_back(msg);
                        }
                        return None;
                    }
                }
            }
        }
    }

    /// Return the number of messages currently in the mailbox
    /// (both in the concurrent queue and save queue).
    pub fn len(&self) -> usize {
        self.save_queue.len() + self.queue.len()
    }

    /// Check if the mailbox is empty.
    pub fn is_empty(&self) -> bool {
        self.save_queue.is_empty() && self.queue.is_empty()
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: The SegQueue is Send+Sync. The save_queue is only accessed by
// the owning process, so we need Send but not Sync for the Mailbox as a whole.
// However, `send()` takes `&self` and only touches the SegQueue, which is Sync.
unsafe impl Send for Mailbox {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_send_receive_basic() {
        let mut mailbox = Mailbox::new();
        mailbox.send(Value::Int(42));
        let msg = mailbox.try_receive();
        assert_eq!(msg, Some(Value::Int(42)));
    }

    #[test]
    fn test_receive_empty() {
        let mut mailbox = Mailbox::new();
        assert_eq!(mailbox.try_receive(), None);
    }

    #[test]
    fn test_send_receive_ordering() {
        let mut mailbox = Mailbox::new();
        mailbox.send(Value::Int(1));
        mailbox.send(Value::Int(2));
        mailbox.send(Value::Int(3));
        assert_eq!(mailbox.try_receive(), Some(Value::Int(1)));
        assert_eq!(mailbox.try_receive(), Some(Value::Int(2)));
        assert_eq!(mailbox.try_receive(), Some(Value::Int(3)));
        assert_eq!(mailbox.try_receive(), None);
    }

    #[test]
    fn test_selective_receive() {
        let mut mailbox = Mailbox::new();
        mailbox.send(Value::Int(1));
        mailbox.send(Value::Int(2));
        mailbox.send(Value::Int(3));

        // Select only the value 2
        let result = mailbox.selective_receive(
            |v| matches!(v, Value::Int(2)),
            None,
        );
        assert_eq!(result, Some(Value::Int(2)));

        // The other messages should still be available
        assert_eq!(mailbox.try_receive(), Some(Value::Int(1)));
        assert_eq!(mailbox.try_receive(), Some(Value::Int(3)));
    }

    #[test]
    fn test_selective_receive_no_match() {
        let mut mailbox = Mailbox::new();
        mailbox.send(Value::Int(1));
        mailbox.send(Value::Int(2));

        let result = mailbox.selective_receive(
            |v| matches!(v, Value::Int(99)),
            None,
        );
        assert_eq!(result, None);

        // Messages should be saved and still available
        assert_eq!(mailbox.try_receive(), Some(Value::Int(1)));
        assert_eq!(mailbox.try_receive(), Some(Value::Int(2)));
    }

    #[test]
    fn test_receive_with_timeout() {
        let mut mailbox = Mailbox::new();
        let start = Instant::now();
        let result = mailbox.receive(Some(Duration::from_millis(50)));
        let elapsed = start.elapsed();
        assert_eq!(result, None);
        assert!(elapsed >= Duration::from_millis(40)); // Allow some slack
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut mailbox = Mailbox::new();
        assert!(mailbox.is_empty());
        assert_eq!(mailbox.len(), 0);

        mailbox.send(Value::Int(1));
        assert!(!mailbox.is_empty());
        assert_eq!(mailbox.len(), 1);

        mailbox.send(Value::Int(2));
        assert_eq!(mailbox.len(), 2);

        mailbox.try_receive();
        assert_eq!(mailbox.len(), 1);
    }

    #[test]
    fn test_concurrent_send() {
        let mailbox = Arc::new(Mailbox::new());
        let mut handles = vec![];

        for i in 0..10 {
            let mb = Arc::clone(&mailbox);
            handles.push(std::thread::spawn(move || {
                mb.send(Value::Int(i));
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(mailbox.len(), 10);
    }
}
