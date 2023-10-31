use crate::matcher::Matcher;
use crate::state::State;
use chrono::{DateTime, Local};
use std::boxed::Box;

pub struct Replacement {
    pub matcher: Box<dyn Matcher>,
    pub date_time: DateTime<Local>,
    pub rest: String,
}

impl Replacement {
    pub fn result(&self, state: &State) -> String {
        let mut name: String =
            self.date_time.format(state.format.as_str()).to_string();

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
    use crate::matcher::{Matcher, Pattern};
    use chrono::TimeZone;
    use regex::Regex;

    fn matcher_with_delimiter(delim: &str) -> Box<dyn Matcher> {
        Box::new(Pattern {
            regex: Regex::new(".").unwrap(),
            name: String::from(""),
            delimiter: String::from(delim),
        })
    }

    fn date(year: i32, month: u32, day: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, 0, 0, 0)
            .earliest()
            .unwrap()
    }

    mod result {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn without_rest() {
            let replacement = Replacement {
                matcher: matcher_with_delimiter(""),
                date_time: date(2023, 10, 25),
                rest: String::from(""),
            };
            let state = State::default();

            assert_eq!("2023-10-25", replacement.result(&state));
        }

        #[test]
        fn with_empty_delim() {
            let replacement = Replacement {
                matcher: matcher_with_delimiter(""),
                date_time: date(2023, 10, 25),
                rest: String::from("foo"),
            };
            let state = State::default();

            assert_eq!("2023-10-25foo", replacement.result(&state));
        }

        #[test]
        fn with_delim() {
            let replacement = Replacement {
                matcher: matcher_with_delimiter("-"),
                date_time: date(2023, 10, 25),
                rest: String::from("foo"),
            };
            let state = State::default();

            assert_eq!("2023-10-25-foo", replacement.result(&state));
        }
    }
}
