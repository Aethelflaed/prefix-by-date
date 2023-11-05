use crate::matcher::Matcher;
use chrono::{DateTime, Local};
use std::boxed::Box;

pub struct Replacement {
    pub matcher: Box<dyn Matcher>,
    pub date_time: DateTime<Local>,
    pub rest: String,
}

impl Replacement {
    pub fn result(&self) -> String {
        let mut name: String = self
            .date_time
            .format(self.matcher.date_format())
            .to_string();

        if !self.rest.is_empty() {
            name.push_str(self.matcher.delimiter());
            name.push_str(&self.rest);
        }

        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::Pattern;
    use chrono::TimeZone;

    fn date_time(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        min: u32,
        sec: u32,
    ) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, hour, min, sec)
            .earliest()
            .unwrap()
    }

    fn date(year: i32, month: u32, day: u32) -> DateTime<Local> {
        date_time(year, month, day, 0, 0, 0)
    }

    mod result {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn without_rest() {
            let replacement = Replacement {
                matcher: Box::<Pattern>::default(),
                date_time: date(2023, 10, 25),
                rest: String::from(""),
            };
            assert_eq!("2023-10-25", replacement.result());
        }

        #[test]
        fn with_empty_delim() {
            let replacement = Replacement {
                matcher: Box::<Pattern>::default(),
                date_time: date(2023, 10, 25),
                rest: String::from("foo"),
            };
            assert_eq!("2023-10-25foo", replacement.result());
        }

        #[test]
        fn with_delim() {
            let mut pattern = Box::<Pattern>::default();
            pattern.delimiter = String::from("-");

            let replacement = Replacement {
                matcher: pattern,
                date_time: date(2023, 10, 25),
                rest: String::from("foo"),
            };
            assert_eq!("2023-10-25-foo", replacement.result());
        }

        #[test]
        fn with_format() {
            let mut pattern = Box::<Pattern>::default();
            pattern.delimiter = String::from("-");
            pattern.format = String::from("%Y-%m-%d-%H");

            let replacement = Replacement {
                matcher: pattern,
                date_time: date_time(2023, 10, 25, 13, 0, 0),
                rest: String::from("foo"),
            };
            assert_eq!("2023-10-25-13-foo", replacement.result());
        }
    }
}
