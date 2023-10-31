use crate::matcher::Matcher;
use crate::replacement::Replacement;
use chrono::{Local, TimeZone};
use regex::{Captures, Regex, RegexBuilder};
use std::any::Any;
use std::str::FromStr;

#[derive(Clone)]
pub struct Pattern {
    pub regex: Regex,
    pub name: String,
    pub delimiter: String,
}

#[derive(Default)]
pub struct PatternBuilder {
    pub regex: String,
    pub name: Option<String>,
    pub delimiter: Option<String>,
}

fn parse<T>(captures: &Captures, name: &str) -> Option<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    captures
        .name(name)
        .map(|str| str.as_str().parse::<T>().unwrap())
}

impl Pattern {
    pub fn builder() -> PatternBuilder {
        PatternBuilder::default()
    }

    pub fn deserialize(name: &str, table: &toml::Table) -> Option<Self> {
        Self::builder().deserialize(name, table)
    }

    fn replacement_from_captures(
        &self,
        captures: Captures,
    ) -> Option<Replacement> {
        let date_time = Local.with_ymd_and_hms(
            parse::<i32>(&captures, "year").unwrap(),
            parse::<u32>(&captures, "month").unwrap(),
            parse::<u32>(&captures, "day").unwrap(),
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

        let rest = elements.join(&self.delimiter);

        date_time.earliest().map(|date_time| Replacement {
            matcher: Box::new(self.clone()),
            date_time,
            rest,
        })
    }
}

impl Matcher for Pattern {
    fn check(&self, file_name: &str) -> Option<Replacement> {
        self.regex
            .captures(file_name)
            .and_then(|captures| self.replacement_from_captures(captures))
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn delimiter(&self) -> &str {
        self.delimiter.as_str()
    }

    fn as_any(&self) -> &dyn Any {
        self
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

    pub fn deserialize(
        &mut self,
        name: &str,
        table: &toml::Table,
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

        self.build()
    }

    pub fn build(&self) -> Option<Pattern> {
        RegexBuilder::new(&self.regex)
            .ignore_whitespace(true)
            .build()
            .ok()
            .map(|regex| Pattern {
                regex,
                name: self.name.clone().unwrap(),
                delimiter: self.delimiter.clone().unwrap_or("".into()),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::LocalResult::Single;
    use pretty_assertions::assert_eq;

    #[test]
    fn invalid_regex() {
        let pattern = Pattern::builder().regex(r"((").name("foo").build();
        assert!(pattern.is_none());
    }

    #[test]
    fn builder() {
        let pattern =
            Pattern::builder().regex(".+").name("foo").build().unwrap();
        assert_eq!(String::from("foo"), pattern.name);
        assert_eq!(String::from(""), pattern.delimiter);

        let pattern2 = Pattern::builder()
            .regex(".+")
            .name("bar")
            .delimiter("-")
            .build()
            .unwrap();
        assert_eq!(String::from("bar"), pattern2.name);
        assert_eq!(String::from("-"), pattern2.delimiter);
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

        let mut name = "IMG-20231028-whatever.jpg";
        let mut replacement = pattern.check(name).unwrap();

        assert_eq!(
            Local.with_ymd_and_hms(2023, 10, 28, 0, 0, 0),
            Single(replacement.date_time)
        );

        assert_eq!(String::from("IMGwhatever.jpg"), replacement.rest);

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

        replacement = pattern.check(name).unwrap();

        assert_eq!(String::from("IMG whatever.jpg"), replacement.rest);

        // Try with a non matching name
        name = "IMG-20230229-smth.jpb";
        assert!(pattern.check(name).is_none());
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
            .build()
            .unwrap();

        let name = "20231028-235959-almost midnight.jpg";
        let replacement = pattern.check(name).unwrap();

        assert_eq!(
            Local.with_ymd_and_hms(2023, 10, 28, 23, 59, 59),
            Single(replacement.date_time)
        );

        assert_eq!(String::from("almost midnight.jpg"), replacement.rest);

        // Invalid date time
        let invalid_name = "20230229-256929-whatever.jpg";
        assert!(pattern.check(invalid_name).is_none());
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

        let name = "skfljdlks-20231028-235959-almost midnight.jpg";
        let replacement = pattern.check(name).unwrap();

        assert_eq!(
            Local.with_ymd_and_hms(2023, 10, 28, 23, 59, 59),
            Single(replacement.date_time)
        );

        assert_eq!(String::from(""), replacement.rest);
    }

    mod deserialize {
        use super::*;
        use pretty_assertions::assert_eq;
        use toml::Table;

        #[test]
        fn empty_map() {
            let table = Table::new();
            assert!(Pattern::deserialize("foo", &table).is_none());
        }

        #[test]
        fn without_regex() {
            let mut table = Table::new();
            table.insert("delimiter".into(), "foo".into());

            assert!(Pattern::deserialize("foo", &table).is_none());
        }

        #[test]
        fn invalid_regex() {
            let mut table = Table::new();
            table.insert("regex".into(), "((".into());

            assert!(Pattern::deserialize("foo", &table).is_none());
        }

        #[test]
        fn simple() {
            let mut table = Table::new();
            table.insert("regex".into(), ".+".into());

            let pattern = Pattern::deserialize("foo", &table).unwrap();

            assert_eq!("foo", pattern.name());
            assert_eq!("", pattern.delimiter());
        }

        #[test]
        fn with_delimiter() {
            let mut table = Table::new();
            table.insert("regex".into(), ".+".into());
            table.insert("delimiter".into(), ".+".into());

            let pattern = Pattern::deserialize("foo", &table).unwrap();

            assert_eq!("foo", pattern.name());
            assert_eq!(".+", pattern.delimiter());
        }
    }
}
