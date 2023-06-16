use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, mpsc, Arc, Mutex},
    thread,
};

pub mod error;
mod threadpool;

type Job<T> = Box<dyn FnOnce() -> T + Send + 'static>;

struct JobData<T> {
    pub job: Job<T>,
    pub result_sender: mpsc::Sender<T>,
}

#[derive(Debug)]
enum WorkerStatus {
    JobDone,
    ThreadExit,
}

pub struct Future<T> {
    result_receiver: Option<mpsc::Receiver<T>>,
}

pub struct ThreadPool<T> {
    current_id: AtomicUsize,
    workers: Arc<Mutex<HashMap<usize, Worker>>>,
    worker_count: Arc<AtomicUsize>,
    working_count: Arc<AtomicUsize>,
    task_sender: Option<mpsc::Sender<JobData<T>>>,
    task_receiver: Arc<Mutex<mpsc::Receiver<JobData<T>>>>,
    worker_status_sender: Option<mpsc::Sender<(usize, WorkerStatus)>>,
    m_thread: Option<thread::JoinHandle<()>>,
    max_size: usize,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}
