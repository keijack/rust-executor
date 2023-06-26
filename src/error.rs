use std::{
    any::Any,
    fmt::{Debug, Display, Result},
};

#[derive(Debug)]
pub enum ErrorKind {
    PoolEnded,
    ResultAlreadyTaken,
    TimeOut,
    TaskRejected,
    TaskCancelled,
    TaskRunning,
    Panic,
}

#[derive(Debug)]
pub struct ExecutorError {
    kind: ErrorKind,
    message: String,
    cause: Option<Box<dyn Any + Send>>,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        let msg = match self {
            ErrorKind::PoolEnded => "PoolEnded",
            ErrorKind::ResultAlreadyTaken => "ResultAlreadyTaken",
            ErrorKind::TaskRejected => "TaskRejected",
            ErrorKind::TaskCancelled => "TaskCancelled",
            ErrorKind::TaskRunning => "TaskStarted",
            ErrorKind::TimeOut => "TimeOUt",
            ErrorKind::Panic => "Panic",
        };
        write!(f, "{:?}", msg)
    }
}

impl ExecutorError {
    pub(crate) fn new(kind: ErrorKind, message: String) -> ExecutorError {
        ExecutorError {
            kind,
            message,
            cause: None,
        }
    }

    pub(crate) fn with_cause(
        kind: ErrorKind,
        message: String,
        cause: Box<dyn Any + Send>,
    ) -> ExecutorError {
        ExecutorError {
            kind,
            message,
            cause: Some(cause),
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &String {
        &self.message
    }

    pub fn cause(&self) -> &Option<Box<dyn Any + Send>> {
        &self.cause
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
