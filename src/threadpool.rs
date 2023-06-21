use super::error::*;
use super::*;
use std::{
    collections::HashMap,
    panic::UnwindSafe,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

impl Builder {
    const DEFALUT_KEEP_ALIVE_SEC: u64 = 300;

    pub fn new() -> Builder {
        Builder {
            core_pool_size: None,
            maximum_pool_size: None,
            exeed_limit_policy: Some(ExceedLimitPolicy::WAIT),
            keep_alive_time: Some(Duration::from_secs(Builder::DEFALUT_KEEP_ALIVE_SEC)),
        }
    }

    pub fn core_pool_size(mut self, size: usize) -> Builder {
        self.core_pool_size = Some(size);
        self
    }

    pub fn maximum_pool_size(mut self, size: usize) -> Builder {
        assert!(size > 0);
        self.maximum_pool_size = Some(size);
        self
    }

    pub fn exeed_limit_policy(mut self, policy: ExceedLimitPolicy) -> Builder {
        self.exeed_limit_policy = Some(policy);
        self
    }

    pub fn keep_alive_time(mut self, keep_alive_time: Duration) -> Builder {
        assert!(!keep_alive_time.is_zero());
        self.keep_alive_time = Some(keep_alive_time);
        self
    }

    pub fn build(self) -> ThreadPool {
        let init_size = match self.core_pool_size {
            Some(size) => size,
            None => 0,
        };
        let max_size = match self.maximum_pool_size {
            Some(size) => size,
            None => usize::MAX,
        };
        let policy = match self.exeed_limit_policy {
            Some(policy) => policy,
            None => ExceedLimitPolicy::WAIT,
        };
        ThreadPool::create(init_size, max_size, policy, self.keep_alive_time)
    }
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        ThreadPool::create(size, size, ExceedLimitPolicy::WAIT, None)
    }

    fn create(
        core_size: usize,
        max_size: usize,
        policy: ExceedLimitPolicy,
        keep_alive_time: Option<Duration>,
    ) -> ThreadPool {
        assert!(max_size > 0);
        assert!(max_size >= core_size);

        let (task_sender, task_receiver) = mpsc::channel();
        let task_receiver = Arc::new(Mutex::new(task_receiver));

        let (task_status_sender, task_status_receiver) = mpsc::channel();

        let mut workers = HashMap::new();
        for id in 0..core_size {
            workers.insert(
                id,
                Worker::new(
                    id,
                    Arc::clone(&task_receiver),
                    None,
                    task_status_sender.clone(),
                ),
            );
        }

        let worker_count = Arc::new(AtomicUsize::new(core_size));
        let working_count = Arc::new(AtomicUsize::new(0));

        let workers = Arc::new(Mutex::new(workers));
        let ws = Arc::clone(&workers);

        let wkc = Arc::clone(&worker_count);
        let wkingc = Arc::clone(&working_count);

        let m_thread = thread::Builder::new()
            .name("thead-pool-cleaner".to_string())
            .spawn(move || loop {
                match task_status_receiver.recv() {
                    Ok(id) => {
                        log::debug!("receive task[#{:?}] status: {:?}", id.0, id.1);
                        match id.1 {
                            WorkerStatus::ThreadExit => {
                                drop(ws.lock().unwrap().remove(&id.0));
                                wkc.fetch_sub(1, Ordering::Relaxed);
                            }
                            WorkerStatus::JobDone => {
                                wkingc.fetch_sub(1, Ordering::Relaxed);
                            }
                        }
                    }
                    Err(_) => {
                        log::debug!("All sender is close, exit this thread.");
                        break;
                    }
                }
            })
            .unwrap();

        ThreadPool {
            current_id: AtomicUsize::new(core_size),
            workers,
            worker_count,
            working_count,
            task_sender: Some(task_sender),
            task_receiver,
            worker_status_sender: Some(task_status_sender),
            m_thread: Some(m_thread),
            max_size,
            policy,
            keep_alive_time,
        }
    }

    pub fn execute<F, T>(&self, f: F) -> Result<Expectation<T>, ExecutorError>
    where
        F: FnOnce() -> T + Send + UnwindSafe + 'static,
        T: Send + 'static,
    {
        let (result_sender, res_receiver) = mpsc::channel();

        let job = move || match std::panic::catch_unwind(f) {
            Ok(res) => {
                if let Err(_) = result_sender.send(res) {
                    log::debug!("Cannot send res to receiver, receiver may close. ");
                }
            }
            Err(err) => {
                log::error!("Run job panic! {:?}", err);
                drop(result_sender);
            }
        };

        let worker_count = self.worker_count.load(Ordering::Relaxed);
        let working_count = self.working_count.load(Ordering::Relaxed);
        log::debug!(
            "workers {}, working {}, max: {}",
            worker_count,
            working_count,
            self.max_size
        );
        if working_count >= self.max_size {
            log::debug!(
                "Working tasks reach the max size. use policy {:?}",
                self.policy
            );
            match self.policy {
                ExceedLimitPolicy::WAIT => {}
                ExceedLimitPolicy::Reject => {
                    return Err(ExecutorError::new(
                        ErrorKind::TaskRejected,
                        "Working tasks reaches to the limit.".to_string(),
                    ));
                }
                ExceedLimitPolicy::CallerRuns => {
                    log::debug!("Run the task at the caller's thread. run now.");
                    job();
                    return Ok(Expectation {
                        result_receiver: Some(res_receiver),
                    });
                }
            };
        }
        if working_count >= worker_count && working_count < self.max_size {
            let mut workers = self.workers.lock().unwrap();
            let id = self.current_id.fetch_add(1, Ordering::Relaxed);
            let task_status_sender = match self.worker_status_sender.clone() {
                Some(sender) => sender,
                None => {
                    return Err(ExecutorError::new(
                        ErrorKind::PoolEnded,
                        "This threadpool is already dropped.".to_string(),
                    ));
                }
            };
            self.worker_count.fetch_add(1, Ordering::Relaxed);
            workers.insert(
                id,
                Worker::new(
                    id,
                    Arc::clone(&self.task_receiver),
                    self.keep_alive_time.clone(),
                    task_status_sender,
                ),
            );
        }
        self.working_count.fetch_add(1, Ordering::Relaxed);

        if let Ok(_) = self.task_sender.as_ref().unwrap().send(Box::new(job)) {
            Ok(Expectation {
                result_receiver: Some(res_receiver),
            })
        } else {
            Err(ExecutorError::new(
                ErrorKind::PoolEnded,
                "Cannot send message to worker thread, This threadpool is already dropped."
                    .to_string(),
            ))
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        log::debug!("Dropping thread pool...");
        // drop the sender, so the receiver in workers will receiv error and then break the loop and join the thread.
        drop(self.task_sender.take());
        // drop the original task sender. After all senders dropped, the cleanr thread will receive error, and then break and joint.
        drop(self.worker_status_sender.take());
        if let Some(thread) = self.m_thread.take() {
            if let Err(_) = thread.join() {}
        }
    }
}

impl Worker {
    fn run_in_thread(
        id: usize,
        task_receiver: Arc<Mutex<mpsc::Receiver<Job>>>,
        wait_time_out: Option<Duration>,
        task_status_sender: mpsc::Sender<(usize, WorkerStatus)>,
    ) {
        loop {
            let job = match task_receiver.lock() {
                // Won't do job inside. Doing job here may cause the lock release after the job done.
                Ok(receiver) => {
                    if let Some(timeout) = wait_time_out {
                        match receiver.recv_timeout(timeout) {
                            Ok(job) => job,
                            Err(_) => {
                                log::debug!("Receive message timeout, release this worker.");
                                break;
                            }
                        }
                    } else {
                        match receiver.recv() {
                            Ok(job) => job,
                            Err(_) => {
                                log::debug!("Chanel sender may disconnect, receive job, error. exit this loop .");
                                break;
                            }
                        }
                    }
                }
                Err(err) => {
                    log::debug!("Cannot lock receiver! close this thread. err {:?}", err);
                    break;
                }
            };

            job();

            if let Err(_) = task_status_sender.send((id, WorkerStatus::JobDone)) {
                log::debug!("Send worder staus error, receiver may close.");
            };
        }
        task_status_sender
            .send((id, WorkerStatus::ThreadExit))
            .unwrap();
    }

    fn new(
        id: usize,
        task_receiver: Arc<Mutex<mpsc::Receiver<Job>>>,
        wait_time_out: Option<Duration>,
        task_status_sender: mpsc::Sender<(usize, WorkerStatus)>,
    ) -> Worker {
        let thread = thread::Builder::new()
            .name("thread-pool-worker-".to_string() + id.to_string().as_str())
            .spawn(move || {
                Worker::run_in_thread(id, task_receiver, wait_time_out, task_status_sender)
            })
            .unwrap();
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(t) = self.thread.take() {
            log::debug!("Dropping worker {:?}", self.id);
            if let Err(err) = t.join() {
                log::debug!("Drop worker... close thread ...error, {:?}", err)
            } else {
                log::debug!("Drop worker {:?} successfully!", self.id);
            }
        }
        log::debug!("Worker {:?} is dropped.", self.id)
    }
}

impl<T> Expectation<T>
where
    T: Send + 'static,
{
    pub fn get_result(&mut self) -> Result<T, ExecutorError> {
        if let Some(receiver) = self.result_receiver.take() {
            log::debug!("start to receive task result!");
            receiver.recv().or(Err(ExecutorError::new(
                ErrorKind::PoolEnded,
                "Cannot send message to worker thread, This threadpool is already dropped."
                    .to_string(),
            )))
        } else {
            log::debug!("Receive result error! Result may be taken!");
            Err(ExecutorError::new(
                ErrorKind::ResultAlreadyTaken,
                "Result is already taken.".to_string(),
            ))
        }
    }

    pub fn get_result_timeout(&mut self, timeout: Duration) -> Result<T, ExecutorError> {
        if let Some(receiver) = self.result_receiver.take() {
            receiver.recv_timeout(timeout).or(Err(ExecutorError::new(
                ErrorKind::TimeOut,
                "Receive result timeout.".to_string(),
            )))
        } else {
            Err(ExecutorError::new(
                ErrorKind::ResultAlreadyTaken,
                "Result is already taken.".to_string(),
            ))
        }
    }
}

impl<T> Drop for Expectation<T> {
    fn drop(&mut self) {
        drop(self.result_receiver.take());
    }
}
