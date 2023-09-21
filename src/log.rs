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
        let mut writer = $log_format.new_writer($timestamp, $severity);
        $crate::log::add_log_pairs!(writer, $($rest)+);
        writer.write($output);
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
            &$value,
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

pub(crate) use add_log_pairs;
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
    /// Constructs the underlying writer.
    pub fn new_writer<W: Write>(
        self,
        timestamp: chrono::DateTime<impl chrono::TimeZone>,
        severity: Severity,
    ) -> Box<dyn LogWriter<W>> {
        match self {
            Self::Json => Box::new(JsonLogWriter::new(timestamp, severity)),
            Self::Text => Box::new(TextLogWriter::new(timestamp, severity)),
        }
    }
}

/// Builds a set of values and writes them to a writer.
pub trait LogWriter<W: Write> {
    /// Adds a new key-value pair.
    fn add(&mut self, name: String, value: &dyn erased_serde::Serialize);

    /// Writes the values to the writer.
    fn write(&self, writer: W);
}

/// Writes the given values in JSON format.
pub struct JsonLogWriter {
    object: serde_json::map::Map<String, serde_json::Value>,
}

impl JsonLogWriter {
    fn new(timestamp: chrono::DateTime<impl chrono::TimeZone>, severity: Severity) -> Self {
        let mut object = serde_json::map::Map::new();
        object.insert(
            "timestamp".to_owned(),
            serde_json::to_value(timestamp).unwrap(),
        );
        object.insert(
            "severity".to_owned(),
            serde_json::to_value(severity).unwrap(),
        );
        Self { object }
    }
}

impl<W: Write> LogWriter<W> for JsonLogWriter {
    fn add(&mut self, name: String, value: &dyn erased_serde::Serialize) {
        self.object
            .insert(name, serde_json::to_value(value).unwrap());
    }

    fn write(&self, writer: W) {
        let mut serializer = serde_json::Serializer::new(writer);
        serde::Serialize::serialize(&self.object, &mut serializer).unwrap();
        writeln!(serializer.into_inner()).unwrap();
    }
}

/// Writes the given values in a pleasing text format.
pub struct TextLogWriter {
    timestamp: chrono::DateTime<chrono::FixedOffset>,
    severity: Severity,
    pairs: Vec<(String, String)>,
}

impl TextLogWriter {
    fn new(timestamp: chrono::DateTime<impl chrono::TimeZone>, severity: Severity) -> Self {
        Self {
            timestamp: timestamp.fixed_offset(),
            severity,
            pairs: Vec::new(),
        }
    }
}

impl<W: Write> LogWriter<W> for TextLogWriter {
    fn add(&mut self, name: String, value: &dyn erased_serde::Serialize) {
        let value_string = TEXT_SERIALIZER.to_string(value).unwrap();
        self.pairs.push((name, value_string))
    }

    fn write(&self, mut writer: W) {
        write!(
            writer,
            "{} [{}]",
            self.timestamp
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            self.severity.as_fixed_length_str()
        )
        .unwrap();
        let mut pairs_iter = self.pairs.iter();
        if let Some((name, value)) = pairs_iter.next() {
            write!(writer, " {} = {}", name, value).unwrap();
        }
        for (name, value) in pairs_iter {
            write!(writer, ", {} = {}", name, value).unwrap();
        }
        writeln!(writer).unwrap();
    }
}

lazy_static! {
    static ref TEXT_SERIALIZER: ron::Options = ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
        .with_default_extension(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES);
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
                its = vec!["the", "final", "countdown"],
                da = 4,
                dada = 5,
                vocalist = Person {
                    name: "Joey Tempest".to_owned(),
                    age: 42
                }
            );
        })?;

        assert_eq!(
            output,
            r#"2023-09-06T00:00:00Z [FATAL] its = ["the","final","countdown"], da = 4, dada = 5, vocalist = (name:"Joey Tempest",age:42)"#
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
