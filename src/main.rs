use std::{path::PathBuf, str::FromStr};

use crate::parsers::parse_state_provinces;

mod parsers;

fn main() {
    let path = PathBuf::from_str("./input/5-Migus Magus.txt").unwrap();
    dbg!(parse_state_provinces(&*path));
}
