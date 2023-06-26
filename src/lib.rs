//!
//! # Rust Threadpool Executor
//!
//! A simple thread pool for running jobs on the worker threads. You can specify the core workers which will live as long as the thread pool , maximum workers which will live with a given keep alive time, and the policy when the jobs submited exceed the maximum size of the workers.
//!
//! ## Usage
//!
//! Create a fix size thread pool and when the job submited will wait when all workers are busy:
//!
//! ```rust
//!
//! let pool = threadpool_executor::ThreadPool::new(1);
//! let mut expectation = pool.execute(|| {"hello, thread pool!"}).unwrap();
//! assert_eq!(expectation.get_result().unwrap(), "hello, thread pool!");
//! ```
//!
//! You can handle wait the result for a specifid time:
//!
//! ```rust
//! let pool = threadpool_executor::ThreadPool::new(1);
//! let r = pool.execute(|| {
//!     std::thread::sleep(std::time::Duration::from_secs(10));
//! });
//! let res = r.unwrap().get_result_timeout(std::time::Duration::from_secs(3));
//! assert!(res.is_err());
//! if let Err(err) = res {
//!     matches!(err.kind(), threadpool_executor::error::ErrorKind::TimeOut);
//! }
//! ```
//!
//! Use `Builder` to create a thread pool:
//!
//! ```rust
//! let pool = threadpool_executor::threadpool::Builder::new()
//!         .core_pool_size(1)
//!         .maximum_pool_size(3)
//!         .keep_alive_time(std::time::Duration::from_secs(300))
//!         .exeed_limit_policy(threadpool_executor::threadpool::ExceedLimitPolicy::Wait)
//!         .build();
//! ```
//!
//! The workers runs in this thread pool will try to catch the `Panic!` using the `std::panic::catch_unwind` in the functions you submit, if catched, the `get_result` method will give you a `Panic` kind ExecutorError.
//!
//! ```rust
//! let pool = threadpool_executor::ThreadPool::new(1);
//! let r = pool.execute(|| {
//!     panic!("panic!!!");
//! });
//! let res = r.unwrap().get_result();
//! assert!(res.is_err());
//! if let Err(err) = res {
//!     matches!(err.kind(), threadpool_executor::error::ErrorKind::Panic);
//! }
//! ```
//! 
//! You can cancel the task when it's waiting in line. The task cannot be cancelled when it's started running. 
//! 
//! ```
//! let pool = threadpool_executor::ThreadPool::new(1);
//! pool.execute(|| {
//!     std::thread::sleep(std::time::Duration::from_secs(3));
//! }).unwrap();
//! let mut exp = pool.execute(|| {}).unwrap();
//! exp.cancel();
//! ```

use crossbeam_channel::{Receiver, Sender};
use std::{
    any::Any,
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

pub mod error;
pub mod threadpool;

pub struct Expectation<T> {
    task_cancelled: Arc<AtomicBool>,
    task_started: Arc<AtomicBool>,
    task_done: Arc<AtomicBool>,
    result_receiver: Option<Receiver<Result<T, Box<dyn Any + Send>>>>,
}

pub struct ThreadPool {
    current_id: AtomicUsize,
    workers: Arc<Mutex<HashMap<usize, threadpool::Worker>>>,
    worker_count: Arc<AtomicUsize>,
    working_count: Arc<AtomicUsize>,
    task_sender: Option<Sender<threadpool::Job>>,
    task_receiver: Receiver<threadpool::Job>,
    worker_status_sender: Option<Sender<(usize, threadpool::WorkerStatus)>>,
    m_thread: Option<thread::JoinHandle<()>>,
    max_size: usize,
    policy: threadpool::ExceedLimitPolicy,
    keep_alive_time: Option<Duration>,
}
