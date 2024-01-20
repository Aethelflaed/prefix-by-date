use std::boxed::Box;

use crate::matcher::{Matcher, Pattern, PredeterminedDate};

pub fn today() -> PredeterminedDate {
    PredeterminedDate::default()
}

pub fn today_boxed() -> Box<dyn Matcher> {
    Box::new(today())
}

pub fn ymd() -> Pattern {
    Pattern::builder()
        .name("ymd")
        .regex(r"(?<start>.+)\s+(?<year>\d{4})(?<month>\d{2})(?<day>\d{2})")
        .build()
        .unwrap()
}

pub fn ymd_boxed() -> Box<dyn Matcher> {
    Box::new(ymd())
}

pub fn weird() -> Pattern {
    Pattern::builder()
        .name("weird")
        .regex("WEIRD")
        .build()
        .unwrap()
}

pub fn weird_boxed() -> Box<dyn Matcher> {
    Box::new(weird())
}
