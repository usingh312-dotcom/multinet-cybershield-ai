//! error — error types for the Net Merge IPC client.

use thiserror::Error;

/// Client-side errors when talking to the Net Merge service over the named pipe.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("pipe disconnected: {0}")]
    PipeDisconnected(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("request timeout: {0}")]
    RequestTimeout(String),
    #[error("mode conflict: {0}")]
    ModeConflict(String),
    #[error("service returned an error: {0}")]
    Service(String),
}

impl From<std::io::Error> for ClientError {
    fn from(value: std::io::Error) -> Self {
        // Distinguish pipe-disconnected errors from generic I/O errors.
        let msg = value.to_string();
        if msg.contains("pipe") || msg.contains("no process") || msg.contains("232") {
            // ERROR_NO_DATA (232) or ERROR_PIPE_NOT_CONNECTED indicate disconnect.
            return ClientError::PipeDisconnected(msg);
        }
        ClientError::Io(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_classified_as_pipe_disconnected() {
        let err = std::io::Error::from_raw_os_error(232); // ERROR_NO_DATA
        let client_err: ClientError = err.into();
        assert!(matches!(client_err, ClientError::PipeDisconnected(_)));
    }

    #[test]
    fn io_error_with_pipe_in_message_classified_as_disconnected() {
        let err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "the pipe has been ended");
        let client_err: ClientError = err.into();
        assert!(matches!(client_err, ClientError::PipeDisconnected(_)));
    }

    #[test]
    fn generic_io_error_remains_io() {
        let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let client_err: ClientError = err.into();
        assert!(matches!(client_err, ClientError::Io(_)));
    }
}
