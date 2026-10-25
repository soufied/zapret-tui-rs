
macro_rules! say {
    () => {
        $crate::diagnostics::events::say(String::new())
    };
    ($($arg:tt)*) => {
        $crate::diagnostics::events::say(format!($($arg)*))
    };
}

pub mod autotune;
pub mod blockcheck;
pub mod dns;
pub mod events;
pub mod http_client;
