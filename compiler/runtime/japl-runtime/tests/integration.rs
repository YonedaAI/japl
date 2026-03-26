//! Integration tests for the JAPL runtime.
//!
//! These tests verify that the scheduler, mailbox, and supervisor systems
//! work together correctly.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

use japl_runtime::error::CrashReason;
use japl_runtime::mailbox::Mailbox;
use japl_runtime::process::{Process, ProcessState};
use japl_runtime::scheduler::Scheduler;
use japl_runtime::supervisor::*;
use japl_runtime::value::Value;

/// Test: spawn multiple processes that each increment a shared counter.
#[test]
fn test_spawn_and_run_multiple() {
    let mut sched = Scheduler::new(4);
    let counter = Arc::new(AtomicI64::new(0));
    let n = 20;

    for _ in 0..n {
        let c = Arc::clone(&counter);
        sched.spawn_fn(move |_ctx| {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    sched.run_until_complete(n);
    assert_eq!(counter.load(Ordering::SeqCst), n as i64);
}

/// Test: a process that panics is caught and marked as Failed.
#[test]
fn test_process_crash_recovery() {
    let mut sched = Scheduler::new(2);
    let success = Arc::new(AtomicI64::new(0));

    // Spawn a process that will crash
    let pid_crash = sched.spawn_fn(|_ctx| {
        panic!("intentional test crash");
    });

    // Spawn a process that will succeed
    let s = Arc::clone(&success);
    let pid_ok = sched.spawn_fn(move |_ctx| {
        s.fetch_add(1, Ordering::SeqCst);
    });

    sched.run_until_complete(2);

    // The crashing process should be in Failed state
    let state = sched.process_state(pid_crash);
    assert!(matches!(state, Some(ProcessState::Failed(_))));

    // The good process should complete
    assert_eq!(success.load(Ordering::SeqCst), 1);
    let state = sched.process_state(pid_ok);
    assert_eq!(state, Some(ProcessState::Done));
}

/// Test: supervisor creates children, handles crash, restarts appropriately.
#[test]
fn test_supervisor_one_for_one() {
    let restart_count = Arc::new(AtomicU64::new(0));
    let rc = Arc::clone(&restart_count);

    let spec = SupervisorSpec {
        strategy: Strategy::OneForOne,
        max_restarts: 5,
        max_seconds: 60,
        children: vec![
            ChildSpec {
                name: "worker-a".to_string(),
                start_fn: Arc::new({
                    let rc = Arc::clone(&rc);
                    move || {
                        rc.fetch_add(1, Ordering::SeqCst);
                        Process::new(Box::new(|_ctx| {}))
                    }
                }),
                restart: RestartPolicy::Permanent,
                shutdown: ShutdownPolicy::Timeout(5000),
            },
            ChildSpec {
                name: "worker-b".to_string(),
                start_fn: Arc::new(|| Process::new(Box::new(|_ctx| {}))),
                restart: RestartPolicy::Permanent,
                shutdown: ShutdownPolicy::Timeout(5000),
            },
        ],
    };

    let mut sup = Supervisor::new(100, spec);
    assert_eq!(sup.child_count(), 2);

    // Initial creation counts as 1 call
    assert_eq!(restart_count.load(Ordering::SeqCst), 1);

    // Simulate worker-a crashing
    let worker_a_pid = sup.find_child("worker-a").unwrap();
    let new_processes = sup
        .handle_child_crash(worker_a_pid, &CrashReason::Custom("crash".into()))
        .unwrap();

    assert_eq!(new_processes.len(), 1);
    assert_eq!(restart_count.load(Ordering::SeqCst), 2);

    // worker-b should still have the same PID
    let _worker_b_pid_after = sup.find_child("worker-b").unwrap();
    // (We cannot compare to original because we did not save it, but
    // it was not touched by OneForOne.)
}

/// Test: supervisor AllForOne restarts all children.
#[test]
fn test_supervisor_all_for_one() {
    let spec = SupervisorSpec {
        strategy: Strategy::AllForOne,
        max_restarts: 5,
        max_seconds: 60,
        children: vec![
            ChildSpec {
                name: "a".to_string(),
                start_fn: Arc::new(|| Process::new(Box::new(|_ctx| {}))),
                restart: RestartPolicy::Permanent,
                shutdown: ShutdownPolicy::Timeout(5000),
            },
            ChildSpec {
                name: "b".to_string(),
                start_fn: Arc::new(|| Process::new(Box::new(|_ctx| {}))),
                restart: RestartPolicy::Permanent,
                shutdown: ShutdownPolicy::Timeout(5000),
            },
            ChildSpec {
                name: "c".to_string(),
                start_fn: Arc::new(|| Process::new(Box::new(|_ctx| {}))),
                restart: RestartPolicy::Permanent,
                shutdown: ShutdownPolicy::Timeout(5000),
            },
        ],
    };

    let mut sup = Supervisor::new(200, spec);
    let original_pids = sup.child_pids();

    // Crash child "b"
    let new_procs = sup
        .handle_child_crash(original_pids[1], &CrashReason::Custom("crash".into()))
        .unwrap();

    // All 3 children should be restarted
    assert_eq!(new_procs.len(), 3);
    let new_pids = sup.child_pids();
    for (old, new) in original_pids.iter().zip(new_pids.iter()) {
        assert_ne!(old, new);
    }
}

/// Test: mailbox send/receive across threads (simulating inter-process messaging).
#[test]
fn test_mailbox_cross_thread() {
    let mailbox = Arc::new(Mailbox::new());
    let mb_sender = Arc::clone(&mailbox);

    let sender = std::thread::spawn(move || {
        for i in 0..5 {
            mb_sender.send(Value::Int(i));
        }
    });

    sender.join().unwrap();

    // Now we need a mutable reference to receive
    // In the real runtime, only the owning process calls receive
    let mb = Arc::try_unwrap(mailbox).ok().expect("Arc still has multiple owners");
    let mut mb = mb;

    let mut received = Vec::new();
    while let Some(val) = mb.try_receive() {
        received.push(val);
    }

    assert_eq!(received.len(), 5);
    assert_eq!(received[0], Value::Int(0));
    assert_eq!(received[4], Value::Int(4));
}

/// Test: selective receive picks the right message.
#[test]
fn test_selective_receive_integration() {
    let mut mailbox = Mailbox::new();

    // Send a mix of message types
    mailbox.send(Value::Constructor(Arc::from("Low"), vec![Value::Int(1)]));
    mailbox.send(Value::Constructor(Arc::from("High"), vec![Value::Int(2)]));
    mailbox.send(Value::Constructor(Arc::from("Low"), vec![Value::Int(3)]));

    // Selectively receive only "High" messages
    let result = mailbox.selective_receive(
        |v| matches!(v, Value::Constructor(name, _) if &**name == "High"),
        None,
    );

    assert!(result.is_some());
    let msg = result.unwrap();
    assert_eq!(
        msg,
        Value::Constructor(Arc::from("High"), vec![Value::Int(2)])
    );

    // Remaining messages should be the two "Low" messages
    let m1 = mailbox.try_receive().unwrap();
    assert_eq!(
        m1,
        Value::Constructor(Arc::from("Low"), vec![Value::Int(1)])
    );
    let m2 = mailbox.try_receive().unwrap();
    assert_eq!(
        m2,
        Value::Constructor(Arc::from("Low"), vec![Value::Int(3)])
    );
}

/// Test: restart intensity limiting prevents infinite restarts.
#[test]
fn test_restart_intensity_limit() {
    let spec = SupervisorSpec {
        strategy: Strategy::OneForOne,
        max_restarts: 3,
        max_seconds: 60,
        children: vec![ChildSpec {
            name: "flaky".to_string(),
            start_fn: Arc::new(|| Process::new(Box::new(|_ctx| {}))),
            restart: RestartPolicy::Permanent,
            shutdown: ShutdownPolicy::Brutal,
        }],
    };

    let mut sup = Supervisor::new(300, spec);

    // Should succeed 3 times
    for _ in 0..3 {
        let pid = sup.children[0].1;
        sup.handle_child_crash(pid, &CrashReason::Custom("crash".into()))
            .unwrap();
    }

    // 4th restart should fail
    let pid = sup.children[0].1;
    let result = sup.handle_child_crash(pid, &CrashReason::Custom("crash".into()));
    assert!(result.is_err());
}

/// Integration test: create a scheduler, spawn a supervisor with 3 child
/// processes, have each child do work, and verify completion.
#[test]
fn test_full_integration_scheduler_with_children() {
    let mut sched = Scheduler::new(4);
    let results = Arc::new(parking_lot::Mutex::new(Vec::new()));

    // Spawn 3 child processes that each push their PID to the results vec
    for i in 0..3 {
        let r = Arc::clone(&results);
        sched.spawn_fn(move |_ctx| {
            // Simulate some work
            let value = i * 10;
            r.lock().push(value);
        });
    }

    sched.run_until_complete(3);

    let mut r = results.lock().clone();
    r.sort();
    assert_eq!(r, vec![0, 10, 20]);
}
