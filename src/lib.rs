use std::{
    any::Any,
    collections::HashMap,
    sync::{atomic::AtomicUsize, mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

pub mod error;
mod threadpool;

#[derive(Debug)]
pub enum ExceedLimitPolicy {
    WAIT,
    Reject,
    CallerRuns,
}

pub struct Expectation<T> {
    result_receiver: Option<mpsc::Receiver<Result<T, Box<dyn Any + Send>>>>,
}

pub struct ThreadPool {
    current_id: AtomicUsize,
    workers: Arc<Mutex<HashMap<usize, Worker>>>,
    worker_count: Arc<AtomicUsize>,
    working_count: Arc<AtomicUsize>,
    task_sender: Option<mpsc::Sender<Job>>,
    task_receiver: Arc<Mutex<mpsc::Receiver<Job>>>,
    worker_status_sender: Option<mpsc::Sender<(usize, WorkerStatus)>>,
    m_thread: Option<thread::JoinHandle<()>>,
    max_size: usize,
    policy: ExceedLimitPolicy,
    keep_alive_time: Option<Duration>,
}

pub struct Builder {
    core_pool_size: Option<usize>,
    maximum_pool_size: Option<usize>,
    exeed_limit_policy: Option<ExceedLimitPolicy>,
    keep_alive_time: Option<Duration>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

#[derive(Debug)]
enum WorkerStatus {
    JobDone,
    ThreadExit,
}
