use crate::application::DEFAULT_DATE_FORMAT;
use crate::matcher::Matcher;
use crate::replacement::Replacement;

use std::str::FromStr;

use chrono::{DateTime, Local, TimeZone};
use regex::{Captures, Regex, RegexBuilder};

#[derive(Clone)]
pub struct Pattern {
    pub regex: Regex,
    pub format: String,
    pub name: String,
    pub delimiter: String,
    pub time: bool,
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            regex: Regex::new(".").expect("Default pattern to compile"),
            format: String::from(DEFAULT_DATE_FORMAT),
            name: String::from(""),
            delimiter: String::from(""),
            time: false,
        }
    }
}

pub struct PatternBuilder {
    pub regex: String,
    pub format: String,
    pub name: Option<String>,
    pub delimiter: Option<String>,
    pub time: Option<bool>,
}

impl Default for PatternBuilder {
    fn default() -> Self {
        Self {
            regex: String::from(""),
            format: String::from(DEFAULT_DATE_FORMAT),
            name: None,
            delimiter: None,
            time: None,
        }
    }
}

struct MatchedDateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: u32,
}

impl MatchedDateTime {
    fn new(captures: &Captures) -> Option<Self> {
        Some(Self {
            year: parse(captures, "year")?,
            month: parse(captures, "month")?,
            day: parse(captures, "day")?,
            hour: parse(captures, "hour").unwrap_or(0),
            min: parse(captures, "min").unwrap_or(0),
            sec: parse(captures, "sec").unwrap_or(0),
        })
    }

    /// Try to return the earliest matching local DateTime corresponding to the
    /// matched date. If it fails, try swapping month and day around to match
    /// imperial date format
    fn resolve(&self) -> Option<DateTime<Local>> {
        match Local
            .with_ymd_and_hms(
                self.year, self.month, self.day, self.hour, self.min, self.sec,
            )
            .earliest()
        {
            Some(time) => Some(time),
            None => Local
                .with_ymd_and_hms(
                    self.year, self.day, self.month, self.hour, self.min,
                    self.sec,
                )
                .earliest(),
        }
    }
}

fn parse<T>(captures: &Captures, name: &str) -> Option<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    captures
        .name(name)
        .and_then(|str| str.as_str().parse::<T>().ok())
}

impl Pattern {
    pub fn builder() -> PatternBuilder {
        PatternBuilder::default()
    }

    pub fn deserialize(
        name: &str,
        table: &toml::Table,
        default_format: &str,
    ) -> Option<Self> {
        Self::builder().deserialize(name, table, default_format)
    }

    pub fn time(&self) -> bool {
        self.time
    }
}

impl Matcher for Pattern {
    fn determine(
        &self,
        replacement: &Replacement,
    ) -> Option<(String, DateTime<Local>)> {
        let captures = self.regex.captures(&replacement.file_stem)?;
        let date_time = MatchedDateTime::new(&captures)?.resolve()?;

        let mut elements = Vec::<String>::default();

        if let Some(start) = captures.name("start") {
            elements.push(start.as_str().into());

            if let Some(end) = captures.name("end") {
                elements.push(end.as_str().into());
            }
        }
        if let Some(text) = captures.name("rest") {
            elements.push(text.as_str().into());
        }

        Some((elements.join(self.delimiter()), date_time))
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn delimiter(&self) -> &str {
        self.delimiter.as_str()
    }

    fn date_format(&self) -> &str {
        self.format.as_str()
    }

    fn auto_accept(&self) -> bool {
        false
    }
}

impl PatternBuilder {
    pub fn regex(&mut self, regex: &str) -> &mut Self {
        self.regex = regex.into();
        self
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name = Some(name.into());
        self
    }

    pub fn delimiter(&mut self, delim: &str) -> &mut Self {
        self.delimiter = Some(delim.into());
        self
    }

    pub fn format(&mut self, format: &str) -> &mut Self {
        self.format = format.into();
        self
    }

    pub fn time(&mut self, time: bool) -> &mut Self {
        self.time = Some(time);
        self
    }

    pub fn deserialize(
        &mut self,
        name: &str,
        table: &toml::Table,
        default_format: &str,
    ) -> Option<Pattern> {
        use toml::Value;

        self.name(name);

        if let Some(regex) = table.get("regex").and_then(Value::as_str) {
            self.regex(regex);
        } else {
            return None;
        }

        if let Some(delim) = table.get("delimiter").and_then(Value::as_str) {
            self.delimiter(delim);
        }

        if let Some(time) = table.get("time").and_then(Value::as_bool) {
            self.time(time);
        }

        if let Some(format) = table.get("format").and_then(Value::as_str) {
            self.format(format);
        } else {
            self.format(default_format);
        }

        self.build()
    }

    pub fn build(&mut self) -> Option<Pattern> {
        RegexBuilder::new(&self.regex)
            .ignore_whitespace(true)
            .build()
            .ok()
            .map(|regex| Pattern {
                regex,
                name: self
                    .name
                    .take()
                    .expect("Name is mandatory to build pattern"),
                delimiter: self.delimiter.take().unwrap_or(" ".into()),
                format: std::mem::take(&mut self.format),
                time: self.time.unwrap_or(false),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{test, assert_eq};

    use std::path::PathBuf;

    #[test]
    fn invalid_regex() {
        let pattern = Pattern::builder().regex(r"((").name("foo").build();
        assert!(pattern.is_none());
    }

    #[test]
    fn builder() {
        let pattern = Pattern::builder()
            .regex(".+")
            .name("foo")
            .format("foo")
            .build()
            .unwrap();
        assert_eq!(String::from("foo"), pattern.name);
        assert_eq!(String::from(" "), pattern.delimiter);
        assert_eq!(String::from("foo"), pattern.format);

        let pattern2 = Pattern::builder()
            .regex(".+")
            .name("bar")
            .delimiter("-")
            .format("%Y-%m")
            .build()
            .unwrap();
        assert_eq!(String::from("bar"), pattern2.name);
        assert_eq!(String::from("-"), pattern2.delimiter);
        assert_eq!(String::from("%Y-%m"), pattern2.format);
    }

    #[test]
    fn pattern_match_start_ymd_end() {
        let pattern = Pattern::builder()
            .regex(
                r"
                (?<start>[A-Z]+)
                -
                (?<year>\d{4})
                (?<month>\d{2})
                (?<day>\d{2})
                -
                (?<end>.+)
                ",
            )
            .name("foo")
            .build()
            .unwrap();

        let name = PathBuf::from("IMG-20231028-whatever.jpg");
        let replacement = pattern.check(&name).unwrap();

        assert_eq!(
            String::from("2023-10-28 IMG whatever"),
            replacement.new_file_stem
        );
    }

    #[test]
    fn pattern_match_start_ydm_end() {
        let pattern = Pattern::builder()
            .regex(
                r"
                (?<start>[A-Z]+)
                -
                (?<year>\d{4})
                (?<month>\d{2})
                (?<day>\d{2})
                -
                (?<end>.+)
                ",
            )
            .name("foo")
            .build()
            .unwrap();

        let name = PathBuf::from("IMG-20232810-whatever.jpg");
        let replacement = pattern.check(&name).unwrap();

        assert_eq!(
            String::from("2023-10-28 IMG whatever"),
            replacement.new_file_stem
        );
    }

    #[test]
    fn pattern_match_start_ymd_end_delimiter() {
        let pattern = Pattern::builder()
            .regex(
                r"
                (?<start>[A-Z]+)
                -
                (?<year>\d{4})
                (?<month>\d{2})
                (?<day>\d{2})
                -
                (?<end>.+)
                ",
            )
            .name("with delim")
            .delimiter(" ")
            .build()
            .unwrap();

        let name = PathBuf::from("IMG-20231028-whatever.jpg");
        let replacement = pattern.check(&name).unwrap();

        assert_eq!(
            String::from("2023-10-28 IMG whatever"),
            replacement.new_file_stem
        );
    }

    #[test]
    fn pattern_match_start_ymd_end_no_match() {
        let pattern = Pattern::builder()
            .regex(
                r"
                (?<start>[A-Z]+)
                -
                (?<year>\d{4})
                (?<month>\d{2})
                (?<day>\d{2})
                -
                (?<end>.+)
                ",
            )
            .name("foo")
            .build()
            .unwrap();

        let name = PathBuf::from("IMG-20230229-smth.jpb");
        assert!(pattern.check(&name).is_none());
    }

    #[test]
    fn pattern_match_ymd_hms_rest() {
        use crate::application::DEFAULT_DATE_TIME_FORMAT;

        let pattern = Pattern::builder()
            .regex(
                r"
                (?<year>\d{4})
                (?<month>\d{2})
                (?<day>\d{2})
                -
                (?<hour>\d{2})
                (?<min>\d{2})
                (?<sec>\d{2})
                -
                (?<rest>.+)
                ",
            )
            .name("test")
            .format(DEFAULT_DATE_TIME_FORMAT)
            .build()
            .unwrap();

        let name = PathBuf::from("20231028-235959-almost midnight.jpg");
        let replacement = pattern.check(&name).unwrap();

        assert_eq!(
            String::from("2023-10-28 23h59m59 almost midnight"),
            replacement.new_file_stem
        );

        // Invalid date time
        let invalid_name = PathBuf::from("20230229-256929-whatever.jpg");
        assert!(pattern.check(&invalid_name).is_none());
    }

    #[test]
    fn pattern_match_ymd_hms() {
        let pattern = Pattern::builder()
            .regex(
                r"
                .+
                -
                (?<year>\d{4})
                (?<month>\d{2})
                (?<day>\d{2})
                -
                (?<hour>\d{2})
                (?<min>\d{2})
                (?<sec>\d{2})
                -
                (?<rest>.+)
                ",
            )
            .name("ymd_hms")
            .build()
            .unwrap();

        let name =
            PathBuf::from("skfljdlks-20231028-235959-almost midnight.jpg");
        let replacement = pattern.check(&name).unwrap();

        assert_eq!(
            String::from("2023-10-28 almost midnight"),
            replacement.new_file_stem
        );
    }

    mod deserialize {
        use super::*;
        use crate::test::{test, assert_eq};
        use toml::Table;

        #[test]
        fn empty_map() {
            let table = Table::new();
            assert!(Pattern::deserialize("foo", &table, "").is_none());
        }

        #[test]
        fn without_regex() {
            let mut table = Table::new();
            table.insert("delimiter".into(), "foo".into());

            assert!(Pattern::deserialize("foo", &table, "").is_none());
        }

        #[test]
        fn invalid_regex() {
            let mut table = Table::new();
            table.insert("regex".into(), "((".into());

            assert!(Pattern::deserialize("foo", &table, "").is_none());
        }

        #[test]
        fn simple() {
            let mut table = Table::new();
            table.insert("regex".into(), ".+".into());

            let pattern = Pattern::deserialize("foo", &table, "").unwrap();

            assert_eq!("foo", pattern.name());
            assert_eq!(" ", pattern.delimiter());
        }

        #[test]
        fn with_format() {
            let mut table = Table::new();
            table.insert("regex".into(), ".+".into());
            table.insert("format".into(), "%Y-%m-%d %Hh%M".into());

            let pattern = Pattern::deserialize("bar", &table, "").unwrap();

            assert_eq!("bar", pattern.name());
            assert_eq!("%Y-%m-%d %Hh%M", pattern.date_format());
        }

        #[test]
        fn with_delimiter() {
            let mut table = Table::new();
            table.insert("regex".into(), ".+".into());
            table.insert("delimiter".into(), ".+".into());

            let pattern = Pattern::deserialize("foo", &table, "").unwrap();

            assert_eq!("foo", pattern.name());
            assert_eq!(".+", pattern.delimiter());
        }
    }
}
