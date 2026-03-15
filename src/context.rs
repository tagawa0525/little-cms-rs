/// Error codes matching C版 `cmsERROR_*` constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ErrorCode {
    Undefined = 0,
    File = 1,
    Range = 2,
    Internal = 3,
    Null = 4,
    Read = 5,
    Seek = 6,
    Write = 7,
    UnknownExtension = 8,
    ColorspaceCheck = 9,
    AlreadyDefined = 10,
    BadSignature = 11,
    CorruptionDetected = 12,
    NotSuitable = 13,
}

#[derive(Debug)]
pub struct CmsError {
    pub code: ErrorCode,
    pub message: String,
}

pub type LogErrorHandler = fn(error_code: ErrorCode, message: &str);

pub struct Context {
    error_handler: Option<LogErrorHandler>,
    pub alarm_codes: [u16; 16],
    pub adaptation_state: f64,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            error_handler: None,
            alarm_codes: [0u16; 16],
            adaptation_state: 1.0,
        }
    }

    pub fn set_error_handler(&mut self, handler: LogErrorHandler) {
        self.error_handler = Some(handler);
    }

    pub fn signal_error(&self, code: ErrorCode, message: &str) {
        if let Some(handler) = self.error_handler {
            handler(code, message);
        }
    }
}

/// Encoded CMS version (re-export from types).
pub use crate::types::VERSION;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]

    fn context_default_state() {
        let ctx = Context::new();
        assert_eq!(ctx.adaptation_state, 1.0);
        assert!(ctx.alarm_codes.iter().all(|&c| c == 0));
    }

    #[test]

    fn signal_error_calls_handler() {
        use std::sync::atomic::{AtomicU32, Ordering};
        static CALLED_CODE: AtomicU32 = AtomicU32::new(999);

        fn handler(code: ErrorCode, _msg: &str) {
            CALLED_CODE.store(code as u32, Ordering::SeqCst);
        }

        let mut ctx = Context::new();
        ctx.set_error_handler(handler);
        ctx.signal_error(ErrorCode::Range, "test error");
        assert_eq!(CALLED_CODE.load(Ordering::SeqCst), ErrorCode::Range as u32);
    }

    #[test]

    fn signal_error_no_handler_no_panic() {
        let ctx = Context::new();
        ctx.signal_error(ErrorCode::Internal, "should not panic");
    }

    #[test]

    fn error_code_values() {
        assert_eq!(ErrorCode::Undefined as u32, 0);
        assert_eq!(ErrorCode::File as u32, 1);
        assert_eq!(ErrorCode::Range as u32, 2);
        assert_eq!(ErrorCode::Internal as u32, 3);
        assert_eq!(ErrorCode::Null as u32, 4);
        assert_eq!(ErrorCode::Read as u32, 5);
        assert_eq!(ErrorCode::Seek as u32, 6);
        assert_eq!(ErrorCode::Write as u32, 7);
        assert_eq!(ErrorCode::UnknownExtension as u32, 8);
        assert_eq!(ErrorCode::ColorspaceCheck as u32, 9);
        assert_eq!(ErrorCode::AlreadyDefined as u32, 10);
        assert_eq!(ErrorCode::BadSignature as u32, 11);
        assert_eq!(ErrorCode::CorruptionDetected as u32, 12);
        assert_eq!(ErrorCode::NotSuitable as u32, 13);
    }
}
