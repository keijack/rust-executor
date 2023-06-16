use std::fmt::{Debug, Display, Result};

#[derive(Debug)]
pub enum ErrorKind {
    PoolEnded,
    ResultAlreadyTaken,
    TimeOut,
    TaskRejected,
}
pub struct ExecutorError {
    kind: ErrorKind,
    message: String,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl ExecutorError {
    pub fn new(kind: ErrorKind, message: String) -> ExecutorError {
        ExecutorError { kind, message }
    }

    fn print(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        write!(
            f,
            "ExecutorError [kind: {}, message: {}]",
            self.kind, self.message
        )
    }
}

impl Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        self.print(f)
    }
}

impl Debug for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        self.print(f)
    }
}
