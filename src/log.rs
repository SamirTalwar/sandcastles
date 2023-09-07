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

#[allow(unused_macros)]
macro_rules! trace {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Trace, $($tokens)+);
    };
}

#[allow(unused_macros)]
macro_rules! debug {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Debug, $($tokens)+);
    };
}

#[allow(unused_macros)]
macro_rules! info {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Info, $($tokens)+);
    };
}

#[allow(unused_macros)]
macro_rules! warning {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Warning, $($tokens)+);
    };
}

#[allow(unused_macros)]
macro_rules! error {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Error, $($tokens)+);
    };
}

#[allow(unused_macros)]
macro_rules! fatal {
    ( $($tokens:tt)+ ) => {
        $crate::log::log!($crate::log::Severity::Fatal, $($tokens)+);
    };
}

macro_rules! log {
    ( $($tokens:tt)+ ) => {
        $crate::log::log_explicitly!(
            std::io::stdout(),
            chrono::offset::Utc::now(),
            $($tokens)+
        );
    };
}

macro_rules! log_explicitly {
    ( $output: expr, $timestamp: expr, $severity: expr, $($rest:tt)+ ) => {{
        use serde::Serialize;
        use std::io::Write;
        let mut values = serde_json::map::Map::new();
        values.insert(
            "timestamp".to_owned(),
            serde_json::to_value($timestamp as chrono::DateTime<_>).unwrap(),
        );
        values.insert(
            "severity".to_owned(),
            serde_json::to_value($severity as $crate::log::Severity).unwrap(),
        );
        $crate::log::log_builder!(values, $($rest)+);
        let mut serializer = serde_json::Serializer::new($output);
        values.serialize(&mut serializer).unwrap();
        writeln!(serializer.into_inner()).unwrap();
    }};
}

macro_rules! log_builder {
    ( $builder:ident, $name: ident = $value:expr, $($rest:tt)* ) => {
        $crate::log::log_builder!($builder, $name = $value);
        $crate::log::log_builder!($builder, $($rest)*);
    };

    ( $builder:ident, $name: ident = $value:expr ) => {
        $builder.insert(
            stringify!($name).to_owned(),
            serde_json::to_value(&$value).unwrap(),
        );
    };

    ( $builder:ident, $name: ident, $($rest:tt)* ) => {
        $crate::log::log_builder!($builder, $name);
        $crate::log::log_builder!($builder, $($rest)*);
    };

    ( $builder:ident, $name: ident ) => {
        $builder.insert(
            stringify!($name).to_owned(),
            serde_json::to_value(&$name).unwrap(),
        );
    };

    ( $builder:ident, ) => {};
}

pub(crate) use log;
pub(crate) use log_builder;
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
        let timestamp = chrono::DateTime::parse_from_rfc3339("2023-09-01T00:00:00Z")?;

        let output = capture_output(|mut buffer| {
            let x = vec![1, 2, 3];
            let y = "hello";
            log_explicitly!(&mut buffer, timestamp, Severity::Debug, x, y);
        })?;

        assert_eq!(
            output,
            r#"{"timestamp":"2023-09-01T00:00:00Z","severity":"DEBUG","x":[1,2,3],"y":"hello"}"#
                .to_owned()
                + "\n"
        );
        Ok(())
    }

    fn capture_output(f: impl FnOnce(&mut dyn std::io::Write)) -> anyhow::Result<String> {
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
}
