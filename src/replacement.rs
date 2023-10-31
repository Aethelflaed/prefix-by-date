use crate::matcher::Matcher;
use chrono::{DateTime, Local};
use std::boxed::Box;

pub struct Replacement {
    pub matcher: Box<dyn Matcher>,
    pub date_time: DateTime<Local>,
    pub rest: String,
}

impl Replacement {}
