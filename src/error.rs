use std::fmt::{Debug, Display, Result};

#[derive(Debug)]
pub enum ErrorKind {
    PoolEnded,
    ResultAlreadyTaken,
    TimeOut,
    TaskRejected,
}

#[derive(Debug)]
pub struct ExecutorError {
    kind: ErrorKind,
    message: String,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        let msg = match self {
            ErrorKind::PoolEnded => "PoolEnded",
            ErrorKind::ResultAlreadyTaken => "ResultAlreadyTaken",
            ErrorKind::TaskRejected => "TaskRejected",
            ErrorKind::TimeOut => "TimeOUt",
        };
        write!(f, "{:?}", msg)
    }
}

impl ExecutorError {
    pub(crate) fn new(kind: ErrorKind, message: String) -> ExecutorError {
        ExecutorError { kind, message }
    }
}

impl Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        write!(
            f,
            "ExecutorError [kind: {}, message: {}]",
            self.kind, self.message
        )
    }
}
