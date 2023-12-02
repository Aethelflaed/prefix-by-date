use crate::matcher::Matcher;
use crate::replacement::Replacement;

use std::path::Path;
use std::str::FromStr;

use chrono::{Local, TimeZone};
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
            format: String::from("%Y-%m-%d"),
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
            format: String::from("%Y-%m-%d"),
            name: None,
            delimiter: None,
            time: None,
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

    fn file_stem_from_captures(&self, captures: Captures) -> Option<String> {
        let date_time = Local.with_ymd_and_hms(
            parse::<i32>(&captures, "year")?,
            parse::<u32>(&captures, "month")?,
            parse::<u32>(&captures, "day")?,
            parse::<u32>(&captures, "hour").unwrap_or(0),
            parse::<u32>(&captures, "min").unwrap_or(0),
            parse::<u32>(&captures, "sec").unwrap_or(0),
        );

        let mut elements = Vec::<String>::default();

        if let Some(text) = captures.name("rest") {
            elements.push(text.as_str().into());
        }
        if let Some(start) = captures.name("start") {
            elements.push(start.as_str().into());

            if let Some(end) = captures.name("end") {
                elements.push(end.as_str().into());
            }
        }

        match date_time.earliest() {
            None => return None,
            Some(time) => {
                elements.push(time.format(self.format.as_str()).to_string())
            }
        }

        elements.rotate_right(1);
        Some(elements.join(&self.delimiter))
    }
}

impl Matcher for Pattern {
    fn check(&self, path: &Path) -> Option<Replacement> {
        let mut replacement = Replacement::try_from(path).ok()?;

        let file_stem = self
            .regex
            .captures(&replacement.str_file_stem()?)
            .and_then(|captures| self.file_stem_from_captures(captures))?;

        replacement.new_file_stem = file_stem;

        Some(replacement)
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

    fn time(&self) -> bool {
        self.time
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
        self.name(name);

        if let Some(toml::Value::String(regex)) = table.get("regex") {
            self.regex(regex.as_str());
        } else {
            return None;
        }

        if let Some(toml::Value::String(delim)) = table.get("delimiter") {
            self.delimiter(delim.as_str());
        }

        if let Some(toml::Value::Boolean(time)) = table.get("time") {
            self.time(*time);
        }

        if let Some(toml::Value::String(format)) = table.get("format") {
            self.format(format.as_str());
        } else {
            self.format(default_format);
        }

        self.build()
    }

    pub fn build(&self) -> Option<Pattern> {
        RegexBuilder::new(&self.regex)
            .ignore_whitespace(true)
            .build()
            .ok()
            .map(|regex| Pattern {
                regex,
                name: self
                    .name
                    .clone()
                    .expect("Name is mandatory to build pattern"),
                delimiter: self.delimiter.clone().unwrap_or(" ".into()),
                format: self.format.clone(),
                time: self.time.unwrap_or(false),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

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
        let mut pattern = Pattern::builder()
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

        let mut name = PathBuf::from("IMG-20231028-whatever.jpg");
        let mut replacement = pattern.check(&name).unwrap();

        assert_eq!(
            String::from("2023-10-28 IMG whatever"),
            replacement.new_file_stem
        );

        // Try again with a delimiter
        pattern = Pattern::builder()
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

        replacement = pattern.check(&name).unwrap();

        assert_eq!(
            String::from("2023-10-28 IMG whatever"),
            replacement.new_file_stem
        );

        // Try with a non matching name
        name = PathBuf::from("IMG-20230229-smth.jpb");
        assert!(pattern.check(&name).is_none());
    }

    #[test]
    fn pattern_match_ymd_hms_rest() {
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
            .format("%Y-%m-%d %Hh%Mm%S")
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
                .+
                ",
            )
            .name("ymd_hms")
            .build()
            .unwrap();

        let name =
            PathBuf::from("skfljdlks-20231028-235959-almost midnight.jpg");
        let replacement = pattern.check(&name).unwrap();

        assert_eq!(String::from("2023-10-28"), replacement.new_file_stem);
    }

    mod deserialize {
        use super::*;
        use pretty_assertions::assert_eq;
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
