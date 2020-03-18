use crate::fmt;

#[derive(Debug, std::cmp::PartialEq)]
pub enum Direction {
    UP,
    DOWN,
    LEFT,
    RIGHT,
    STOP,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}