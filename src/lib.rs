use std::{
    any::Any,
    collections::HashMap,
    sync::{atomic::AtomicUsize, mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

pub mod error;
pub mod threadpool;

pub struct Expectation<T> {
    result_receiver: Option<mpsc::Receiver<Result<T, Box<dyn Any + Send>>>>,
}

pub struct ThreadPool {
    current_id: AtomicUsize,
    workers: Arc<Mutex<HashMap<usize, threadpool::Worker>>>,
    worker_count: Arc<AtomicUsize>,
    working_count: Arc<AtomicUsize>,
    task_sender: Option<mpsc::Sender<threadpool::Job>>,
    task_receiver: Arc<Mutex<mpsc::Receiver<threadpool::Job>>>,
    worker_status_sender: Option<mpsc::Sender<(usize, threadpool::WorkerStatus)>>,
    m_thread: Option<thread::JoinHandle<()>>,
    max_size: usize,
    policy: threadpool::ExceedLimitPolicy,
    keep_alive_time: Option<Duration>,
}
