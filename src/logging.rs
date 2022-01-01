pub extern crate colored;
pub extern crate chrono;

pub use chrono::Local;


#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        use $crate::colored::*;
        let msg = format!($($arg)*);
        let now = format!("{}",$crate::Local::now().format("[%Y-%m-%d][%H:%M:%S]"));
        let error = format!("{} [ERROR] {}", now, msg).red();
        eprintln!("{}", error);
    }}
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {{
        use $crate::colored::*;
        let msg = format!($($arg)*);
        let now = format!("{}",$crate::Local::now().format("[%Y-%m-%d][%H:%M:%S]"));
        let error = format!("{} [WARNING] {}", now, msg).yellow();
        eprintln!("{}", error);
    }}
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        use $crate::colored::*;
        let msg = format!($($arg)*);
        let now = format!("{}",$crate::Local::now().format("[%Y-%m-%d][%H:%M:%S]"));
        let error = format!("{} [INFO] {}", now, msg).white();
        eprintln!("{}", error);
    }
}
