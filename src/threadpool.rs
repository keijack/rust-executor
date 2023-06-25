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

pub(super) type Job = Box<dyn FnOnce() + Send + 'static>;

///
/// The policy that can be set when the task submited exceed the maximum size of the `Threadpool`.
///
#[derive(Debug)]
pub enum ExceedLimitPolicy {
    ///
    /// The task will wait until some workers are idle.
    ///
    Wait,
    ///
    /// The task will be rejected, A `TaskRejected` `ExecutorError` will be given.
    ///
    /// ```
    /// let pool = executor::threadpool::Builder::new()
    ///         .core_pool_size(1)
    ///         .maximum_pool_size(1)
    ///         .exeed_limit_policy(executor::threadpool::ExceedLimitPolicy::Reject)
    ///         .build();
    /// let res = pool.execute(|| {
    ///         std::thread::sleep(std::time::Duration::from_secs(10));
    /// });
    /// assert!(res.is_ok());
    /// let res = pool.execute(|| "a");
    /// assert!(res.is_err());
    /// if let Err(err) = res {
    ///         matches!(err.kind(), executor::error::ErrorKind::TaskRejected);
    /// }
    /// ```
    ///
    Reject,
    ///
    /// The task will be run in the caller's thread, and will run immediatly.
    ///
    CallerRuns,
}

pub struct Builder {
    core_pool_size: Option<usize>,
    maximum_pool_size: Option<usize>,
    exeed_limit_policy: Option<ExceedLimitPolicy>,
    keep_alive_time: Option<Duration>,
}

impl Builder {
    const DEFALUT_KEEP_ALIVE_SEC: u64 = 300;

    ///
    /// A builder use to build a `ThreadPool`
    ///
    /// # Example
    ///
    /// ```
    /// let pool = executor::threadpool::Builder::new()
    /// .core_pool_size(1)
    /// .maximum_pool_size(3)
    /// .keep_alive_time(std::time::Duration::from_secs(300)) // None-core-thread keep_alive_time, default value is 5 minutes.
    /// .exeed_limit_policy(executor::threadpool::ExceedLimitPolicy::Reject) // Default value is Wait.
    /// .build();
    /// ```
    ///
    pub fn new() -> Builder {
        Builder {
            core_pool_size: None,
            maximum_pool_size: None,
            exeed_limit_policy: Some(ExceedLimitPolicy::Wait),
            keep_alive_time: Some(Duration::from_secs(Builder::DEFALUT_KEEP_ALIVE_SEC)),
        }
    }

    ///
    /// Core threads will run until the threadpool dropped.
    ///
    /// # Example
    ///
    /// ```
    /// let pool = executor::threadpool::Builder::new()
    /// .core_pool_size(1)
    /// .build();
    /// ```
    ///
    pub fn core_pool_size(mut self, size: usize) -> Builder {
        self.core_pool_size = Some(size);
        self
    }

    ///
    /// Maximum threads that run in this threadpool, include the core threads,
    /// the size of the none-core threads = `maximum_pool_size - core_pool_size`.
    ///
    /// None core threads will live with a given `keep_alive_time`. If the `keep_alive_time`
    /// is not set, it will default to 5 minutes.
    ///
    /// # Example
    ///
    /// ```
    /// let pool = executor::threadpool::Builder::new()
    /// .core_pool_size(1)
    /// .maximum_pool_size(3)
    /// .build();
    /// ```
    ///
    pub fn maximum_pool_size(mut self, size: usize) -> Builder {
        assert!(size > 0);
        self.maximum_pool_size = Some(size);
        self
    }

    ///
    /// When the threads are all working, the new tasks coming will follow the given policy
    ///
    /// # Example
    ///
    /// ```
    /// let pool = executor::threadpool::Builder::new()
    /// .core_pool_size(1)
    /// .maximum_pool_size(1)
    /// .exeed_limit_policy(executor::threadpool::ExceedLimitPolicy::Reject)
    /// .build();
    /// let res = pool.execute(|| {
    ///     std::thread::sleep(std::time::Duration::from_secs(3));
    /// });
    /// assert!(res.is_ok());
    /// let res = pool.execute(|| "a");
    /// assert!(res.is_err());
    /// if let Err(err) = res {
    ///     matches!(err.kind(), executor::error::ErrorKind::TaskRejected);
    /// }
    /// ```
    ///
    pub fn exeed_limit_policy(mut self, policy: ExceedLimitPolicy) -> Builder {
        self.exeed_limit_policy = Some(policy);
        self
    }

    ///
    /// None core threads will live with a given `keep_alive_time`. If the `keep_alive_time`
    /// is not set, it will default to 5 minutes.
    ///
    /// # Example
    ///
    /// ```
    /// let pool = executor::threadpool::Builder::new()
    /// .core_pool_size(1)
    /// .maximum_pool_size(3)
    /// .keep_alive_time(std::time::Duration::from_secs(60))
    /// .build();
    /// ```
    ///
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
            None => ExceedLimitPolicy::Wait,
        };
        ThreadPool::create(init_size, max_size, policy, self.keep_alive_time)
    }
}

impl ThreadPool {
    ///
    /// Create a fix size thread pool with the Polic `ExceedLimitPolicy::Wait`
    ///
    /// # Example
    ///
    /// ```
    /// use executor::ThreadPool;
    ///
    /// let pool = ThreadPool::new(1);
    /// pool.execute(|| {println!("hello, world!");});
    /// ```
    ///
    pub fn new(size: usize) -> ThreadPool {
        ThreadPool::create(size, size, ExceedLimitPolicy::Wait, None)
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

    ///
    /// Execute a closure in the threadpool, return a `Result` indicating whether the `submit` operation succeeded or not.
    ///
    /// `Submit` operation will fail when the pool reach to `the maximum_pool_size` and the `exeed_limit_policy` is set to `Reject`.
    ///
    /// You can get a `Expectation<T>` when `Result` is `Ok`, `T` here is the return type of your closure.
    ///
    /// You can use `get_result` or `get_result_timeout` method in the `Expectation` object to get the result of your closure. The
    /// two method above will block when the result is returned or timeout.
    ///
    /// `Expectation::get_result` and `Expectation::get_result_timeout` return a `Result` which will return the return value of your
    /// closure when `Ok`, and `Err` will be returned when your closure `panic`.
    ///
    /// # Example
    ///
    /// ```
    /// let pool = executor::ThreadPool::new(1);
    /// let exp = pool.execute(|| 1 + 2);
    /// assert_eq!(exp.unwrap().get_result().unwrap(), 3);
    /// ```
    ///
    /// When `panic`:
    ///
    /// ```
    /// let pool = executor::ThreadPool::new(1);
    /// let exp = pool.execute(|| {
    ///     panic!("panic!!!");
    /// });
    /// let res = exp.unwrap().get_result();
    /// assert!(res.is_err());
    /// if let Err(err) = res {
    ///     matches!(err.kind(), executor::error::ErrorKind::Panic);
    /// }
    /// ```
    ///
    pub fn execute<F, T>(&self, f: F) -> Result<Expectation<T>, ExecutorError>
    where
        F: FnOnce() -> T + Send + UnwindSafe + 'static,
        T: Send + 'static,
    {
        let (result_sender, result_receiver) = mpsc::channel();

        let job = move || {
            if let Err(_) = result_sender.send(std::panic::catch_unwind(f)) {
                log::debug!("Cannot send res to receiver, receiver may close. ");
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
                ExceedLimitPolicy::Wait => {}
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
                        result_receiver: Some(result_receiver),
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
                result_receiver: Some(result_receiver),
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

pub(super) struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

#[derive(Debug)]
pub(super) enum WorkerStatus {
    JobDone,
    ThreadExit,
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
        if let Err(_) = task_status_sender.send((id, WorkerStatus::ThreadExit)) {
            log::debug!("Send worder staus error, receiver may close.");
        }
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
    /// This method returns a `Result` which will return the return value of your
    /// closure when `Ok`, and `Err` will be returned when your closure `panic`.
    ///
    /// # Example
    ///
    /// ```
    /// let pool = executor::ThreadPool::new(1);
    /// let exp = pool.execute(|| 1 + 2);
    /// assert_eq!(exp.unwrap().get_result().unwrap(), 3);
    /// ```
    ///
    /// When `panic`:
    ///
    /// ```
    /// let pool = executor::ThreadPool::new(1);
    /// let exp = pool.execute(|| {
    ///     panic!("panic!!!");
    /// });
    /// let res = exp.unwrap().get_result();
    /// assert!(res.is_err());
    /// if let Err(err) = res {
    ///     matches!(err.kind(), executor::error::ErrorKind::Panic);
    /// }
    /// ```
    ///
    pub fn get_result(&mut self) -> Result<T, ExecutorError> {
        if let Some(receiver) = self.result_receiver.take() {
            match receiver.recv() {
                Ok(res) => match res {
                    Ok(res) => Ok(res),
                    Err(cause) => Err(ExecutorError::with_cause(
                        ErrorKind::Panic,
                        "Function panic!".to_string(),
                        cause,
                    )),
                },
                Err(_) => Err(ExecutorError::new(
                    ErrorKind::PoolEnded,
                    "Cannot send message to worker thread, This threadpool is already dropped."
                        .to_string(),
                )),
            }
        } else {
            log::debug!("Receive result error! Result may be taken!");
            Err(ExecutorError::new(
                ErrorKind::ResultAlreadyTaken,
                "Result is already taken.".to_string(),
            ))
        }
    }

    /// This method returns a `Result` which will return the return value of your
    /// closure when `Ok`, and `Err` will be returned when your closure `panic` or
    /// `timeout`.
    ///
    /// # Example
    ///
    /// ```
    /// let pool = executor::ThreadPool::new(1);
    /// let exp = pool.execute(|| 1 + 2);
    /// assert_eq!(exp.unwrap().get_result_timeout(std::time::Duration::from_secs(1)).unwrap(), 3);
    /// ```
    ///
    /// When `timeout`:
    ///
    /// ```
    /// use std::time::Duration;
    /// let pool = executor::ThreadPool::new(1);
    /// let exp = pool.execute(|| {
    ///     std::thread::sleep(Duration::from_secs(3));
    /// });
    /// let res = exp.unwrap().get_result_timeout(Duration::from_secs(1));
    /// assert!(res.is_err());
    /// if let Err(err) = res {
    ///     matches!(err.kind(), executor::error::ErrorKind::TimeOut);
    /// }
    /// ```
    ///
    pub fn get_result_timeout(&mut self, timeout: Duration) -> Result<T, ExecutorError> {
        if let Some(receiver) = self.result_receiver.take() {
            match receiver.recv_timeout(timeout) {
                Ok(res) => match res {
                    Ok(res) => Ok(res),
                    Err(cause) => Err(ExecutorError::with_cause(
                        ErrorKind::Panic,
                        "Function panic!".to_string(),
                        cause,
                    )),
                },
                Err(_) => Err(ExecutorError::new(
                    ErrorKind::TimeOut,
                    "Receive result timeout.".to_string(),
                )),
            }
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
