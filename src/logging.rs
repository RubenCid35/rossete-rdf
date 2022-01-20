pub extern crate colored;
pub extern crate chrono;

pub use chrono::Local;
use std::time;


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
        let msg = format!($($arg)*);
        let error = format!("{} [INFO] {}", $crate::Local::now().format("[%Y-%m-%d][%H:%M:%S]"), msg);
        eprintln!("{}", error);
    }
}

pub fn time_info(procedure: &str, start_time: time::Instant){
    use colored::*;
    let error = format!("{} [PERFORMANCE INFO] The Procedure: \"{}\" Requiered the following time: {} ms", Local::now().format("[%Y-%m-%d][%H:%M:%S]"), procedure, start_time.elapsed().as_millis()).green();
    eprintln!("{}", error);
}