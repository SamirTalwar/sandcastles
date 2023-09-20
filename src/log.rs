//! This is a simple library used for JSON logging.
//!
//! Use as follows:
//!
//! ```ignore
//! log!(Severity::Debug, event = "SERVICE_STARTED", port = 8080)
//! ```
//!
//! Any value serializable by `serde` can be logged.
//!
//! If the name and value are the same, you can pass it by name:
//!
//! ```ignore
//! log!(Severity::Info, request)
//! ```
//!
//! There are helpers for the various severity levels:
//!
//! ```ignore
//! warning!(code = "DUPLICATE", value = duplicate_value)
//! ```
//!
//! You can log errors too. They also need to be serializable, but there is a
//! helper method, `log()`, for `std::io::Error` and a few other types:
//!
//! ```ignore
//! error!(code = "OHNOITBROKE", error = err.log())
//! ```
//!
//! Everything else is up to you.

use std::io::Write;
use std::sync::RwLock;

use lazy_static::lazy_static;

pub trait Loggable {
    type Serialized;

    fn log(&self) -> Self::Serialized;
}

impl Loggable for std::io::Error {
    type Serialized = LoggableIoError;

    fn log(&self) -> Self::Serialized {
        self.into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LoggableIoError {
    kind: String,
    message: String,
}

impl From<&std::io::Error> for LoggableIoError {
    fn from(value: &std::io::Error) -> Self {
        Self {
            kind: format!("{:?}", value.kind()),
            message: value.to_string(),
        }
    }
}

impl From<std::io::Error> for LoggableIoError {
    fn from(value: std::io::Error) -> Self {
        (&value).into()
    }
}

impl std::fmt::Display for LoggableIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)
    }
}

/// Severity levels, for logging.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    fn as_fixed_length_str(&self) -> &'static str {
        match self {
            Severity::Trace => "TRACE",
            Severity::Debug => "DEBUG",
            Severity::Info => "INFO ",
            Severity::Warning => "WARN ",
            Severity::Error => "ERROR",
            Severity::Fatal => "FATAL",
        }
    }
}

/// Log at TRACE severity.
///
/// ```ignore
/// trace!(name = "value", ...)
/// ```
#[allow(unused_macros)]
macro_rules! trace {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Trace, $($tokens)+)
    };
}

/// Log at DEBUG severity.
///
/// ```ignore
/// debug!(name = "value", ...)
/// ```
#[allow(unused_macros)]
macro_rules! debug {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Debug, $($tokens)+)
    };
}

/// LOG at INFO severity.
///
/// ```ignore
/// info!(name = "value", ...)
/// ```
#[allow(unused_macros)]
macro_rules! info {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Info, $($tokens)+)
    };
}

/// LOG at WARNING severity.
///
/// ```ignore
/// warning!(name = "value", ...)
/// ```
#[allow(unused_macros)]
macro_rules! warning {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Warning, $($tokens)+)
    };
}

/// LOG at ERROR severity.
///
/// ```ignore
/// error!(name = "value", ...)
/// ```
#[allow(unused_macros)]
macro_rules! error {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Error, $($tokens)+)
    };
}

/// LOG at FATAL severity.
///
/// ```ignore
/// fatal!(name = "value", ...)
/// ```
#[allow(unused_macros)]
macro_rules! fatal {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Fatal, $($tokens)+)
    };
}

/// Log the values given, along with the current time and given severity.
///
/// All values must be serializable with `serde`.
///
/// ```ignore
/// log!(Severity::Debug, name = "value", ...)
/// ```
macro_rules! log {
    ( $($tokens:tt)+ ) => {
        $crate::log::log_explicitly!(
            std::io::stderr(),
            $crate::log::global_log_format(),
            chrono::offset::Utc::now(),
            $($tokens)+
        )
    };
}

/// Internal; subject to change.
///
/// ```ignore
/// log_explicitly!(stderr(), now(), Level::INFO, name = "value", ...)
/// ```
#[doc(hidden)]
macro_rules! log_explicitly {
    ( $output: expr, $log_format: expr, $timestamp: expr, $severity: expr, $($rest:tt)+ ) => {{
        #[allow(unused_imports)]
        use $crate::log::Loggable;
        #[allow(clippy::vec_init_then_push)]
        let mut pairs = $crate::log::Pairs::with_capacity($crate::log::count_pairs!($($rest)+));
        $crate::log::add_log_pairs!(pairs, $($rest)+);
        $log_format.write($output, $timestamp, $severity, pairs);
    }};
}

#[doc(hidden)]
macro_rules! add_log_pairs {
    // Adds the name/value pair to the builder, and proceeds.
    //
    //     add_log_pairs!(builder, name = "value", ...)
    ( $builder:ident, $name: ident = $value:expr, $($rest:tt)* ) => {
        $crate::log::add_log_pairs!($builder, $name = $value);
        $crate::log::add_log_pairs!($builder, $($rest)*)
    };

    // Adds the name/value pair to the builder, and stops.
    //
    //     add_log_pairs!(builder, name = "value")
    ( $builder:ident, $name: ident = $value:expr ) => {
        $builder.add(
            stringify!($name).to_owned(),
            serde_json::to_value(&$value).unwrap(),
        );
    };

    // Adds the value to the builder, using its name, and proceeds.
    //
    //     add_log_pairs!(builder, name, ...)
    ( $builder:ident, $name: ident, $($rest:tt)* ) => {
        $crate::log::add_log_pairs!($builder, $name);
        $crate::log::add_log_pairs!($builder, $($rest)*)
    };

    // Adds the value to the builder, using its name, and stops.
    //
    //     add_log_pairs!(builder, name)
    ( $builder:ident, $name: ident ) => {
        $crate::log::add_log_pairs!($builder, $name = $name);
    };

    // If the user leaves a trailing comma, this swallows it.
    ( $builder:ident, ) => {};
}

// Counts the pairs.
#[doc(hidden)]
macro_rules! count_pairs {
    ( $name:ident = $value:expr, $($rest:tt)* ) => {
        1 + $crate::log::count_pairs!($($rest)*)
    };

    ( $name: ident = $value:expr ) => { 1 };

    ( $name: ident, $($rest:tt)* ) => {
        1 + $crate::log::count_pairs!($($rest)*)
    };

    ( $name: ident ) => { 1 };

    ( , ) => { 0 };

    ( ) => { 0 };
}

pub(crate) use add_log_pairs;
pub(crate) use count_pairs;
pub(crate) use log;
pub(crate) use log_explicitly;

#[allow(unused_imports)]
pub(crate) use debug;
#[allow(unused_imports)]
pub(crate) use error;
#[allow(unused_imports)]
pub(crate) use fatal;
#[allow(unused_imports)]
pub(crate) use info;
#[allow(unused_imports)]
pub(crate) use trace;
#[allow(unused_imports)]
pub(crate) use warning;

/// The textual format used when writing.
#[derive(Debug, Clone, Copy)]
pub enum LogFormat {
    Json,
    Text,
}

impl LogFormat {
    /// Writes the given items to the log output in the format specified.
    pub fn write(
        self,
        mut output: impl Write,
        timestamp: chrono::DateTime<impl chrono::TimeZone>,
        severity: Severity,
        pairs: Pairs,
    ) {
        match self {
            Self::Json => {
                let mut values = serde_json::map::Map::new();
                values.insert(
                    "timestamp".to_owned(),
                    serde_json::to_value(timestamp).unwrap(),
                );
                values.insert(
                    "severity".to_owned(),
                    serde_json::to_value(severity).unwrap(),
                );
                values.extend(pairs);
                let mut serializer = serde_json::Serializer::new(output);
                serde::Serialize::serialize(&values, &mut serializer).unwrap();
                writeln!(serializer.into_inner()).unwrap();
            }
            Self::Text => {
                write!(
                    output,
                    "{} [{}]",
                    timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    severity.as_fixed_length_str()
                )
                .unwrap();
                let mut pairs_iter = pairs.into_iter();
                if let Some((name, value)) = pairs_iter.next() {
                    write!(output, " {} = {}", name, value).unwrap();
                }
                for (name, value) in pairs_iter {
                    write!(output, ", {} = {}", name, value).unwrap();
                }
                writeln!(output).unwrap();
            }
        }
    }
}

pub struct Pairs(Vec<(String, serde_json::Value)>);

impl Pairs {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Adds a new pair to the list of pairs.
    pub fn add(&mut self, name: String, value: serde_json::Value) {
        self.0.push((name, value));
    }
}

impl IntoIterator for Pairs {
    type Item = (String, serde_json::Value);

    type IntoIter = <Vec<(std::string::String, serde_json::Value)> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

lazy_static! {
    static ref LOG_FORMAT: RwLock<LogFormat> = RwLock::new(detect_log_format());
}

fn detect_log_format() -> LogFormat {
    if std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        LogFormat::Text
    } else {
        LogFormat::Json
    }
}

pub fn global_log_format() -> LogFormat {
    *LOG_FORMAT.read().unwrap()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::BufWriter;

    use super::*;

    #[test]
    fn test_logging_data() -> anyhow::Result<()> {
        let timestamp = chrono::DateTime::parse_from_rfc3339("2023-09-01T00:00:00Z")?;

        let output = capture_output(|mut buffer| {
            log_explicitly!(
                &mut buffer,
                LogFormat::Json,
                timestamp,
                Severity::Debug,
                a = 1,
                b = "two",
                c = 3.0,
            );
        })?;

        assert_eq!(
            output,
            r#"{"timestamp":"2023-09-01T00:00:00Z","severity":"DEBUG","a":1,"b":"two","c":3.0}"#
                .to_owned()
                + "\n"
        );
        Ok(())
    }

    #[test]
    fn test_logging_nested_data() -> anyhow::Result<()> {
        let timestamp = chrono::DateTime::parse_from_rfc3339("2023-09-02T00:00:00Z")?;

        let output = capture_output(|mut buffer| {
            log_explicitly!(
                &mut buffer,
                LogFormat::Json,
                timestamp,
                Severity::Info,
                numbers = vec![vec![1, 2], vec![3, 4]],
                dictionary = BTreeMap::from([
                    ("apple", "Apfel"),
                    ("banana", "Banane"),
                    ("carrot", "Rüebli")
                ]),
                person = Person {
                    name: "Alice".to_owned(),
                    age: 21
                },
                people = vec![
                    Person {
                        name: "Bob".to_owned(),
                        age: 32
                    },
                    Person {
                        name: "Carol".to_owned(),
                        age: 43
                    }
                ],
            );
        })?;

        assert_eq!(
            output,
            r#"{"timestamp":"2023-09-02T00:00:00Z","severity":"INFO","numbers":[[1,2],[3,4]],"dictionary":{"apple":"Apfel","banana":"Banane","carrot":"Rüebli"},"person":{"name":"Alice","age":21},"people":[{"name":"Bob","age":32},{"name":"Carol","age":43}]}"#.to_owned() + "\n"
        );
        Ok(())
    }

    #[test]
    fn test_logging_by_name_only() -> anyhow::Result<()> {
        let timestamp = chrono::DateTime::parse_from_rfc3339("2023-09-03T00:00:00Z")?;

        let output = capture_output(|mut buffer| {
            let x = vec![1, 2, 3];
            let y = "hello";
            log_explicitly!(
                &mut buffer,
                LogFormat::Json,
                timestamp,
                Severity::Warning,
                x,
                y
            );
        })?;

        assert_eq!(
            output,
            r#"{"timestamp":"2023-09-03T00:00:00Z","severity":"WARNING","x":[1,2,3],"y":"hello"}"#
                .to_owned()
                + "\n"
        );
        Ok(())
    }

    #[test]
    fn test_logging_errors() -> anyhow::Result<()> {
        let timestamp = chrono::DateTime::parse_from_rfc3339("2023-09-04T00:00:00Z")?;

        let output = capture_output(|mut buffer| {
            let error = Whoops {
                code: "WHOOPS".to_owned(),
                message: "Uh oh.".to_owned(),
            };
            log_explicitly!(
                &mut buffer,
                LogFormat::Json,
                timestamp,
                Severity::Error,
                error
            );
        })?;

        assert_eq!(
            output,
            r#"{"timestamp":"2023-09-04T00:00:00Z","severity":"ERROR","error":{"code":"WHOOPS","message":"Uh oh."}}"#
                .to_owned()
                + "\n"
        );
        Ok(())
    }

    #[test]
    fn test_logging_io_errors() -> anyhow::Result<()> {
        let timestamp = chrono::DateTime::parse_from_rfc3339("2023-09-05T00:00:00Z")?;

        let output = capture_output(|mut buffer| {
            let error = std::io::Error::new(std::io::ErrorKind::TimedOut, "it took too long");
            log_explicitly!(
                &mut buffer,
                LogFormat::Json,
                timestamp,
                Severity::Error,
                error = error.log()
            );
        })?;

        assert_eq!(
            output,
            r#"{"timestamp":"2023-09-05T00:00:00Z","severity":"ERROR","error":{"kind":"TimedOut","message":"it took too long"}}"#
                .to_owned()
                + "\n"
        );
        Ok(())
    }

    #[test]
    fn test_logging_for_reading() -> anyhow::Result<()> {
        let timestamp = chrono::DateTime::parse_from_rfc3339("2023-09-06T00:00:00Z")?;

        let output = capture_output(|mut buffer| {
            log_explicitly!(
                &mut buffer,
                LogFormat::Text,
                timestamp,
                Severity::Fatal,
                its = ["the", "final", "countdown"],
                da = 4,
                dada = 5,
            );
        })?;

        assert_eq!(
            output,
            r#"2023-09-06T00:00:00Z [FATAL] its = ["the","final","countdown"], da = 4, dada = 5"#
                .to_owned()
                + "\n"
        );
        Ok(())
    }

    fn capture_output(f: impl FnOnce(&mut dyn Write)) -> anyhow::Result<String> {
        let mut buffer = BufWriter::new(Vec::new());
        f(&mut buffer);
        let output = String::from_utf8(buffer.into_inner()?)?;
        Ok(output)
    }

    #[derive(Debug, serde::Serialize)]
    struct Person {
        name: String,
        age: u8,
    }

    #[derive(Debug, serde::Serialize)]
    struct Whoops {
        code: String,
        message: String,
    }

    impl std::fmt::Display for Whoops {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Whoops [{}]: {}", self.code, self.message)
        }
    }

    impl std::error::Error for Whoops {}
}
