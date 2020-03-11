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

// implement from usize for Direction to be able to use an AtomicUsize and load the Direction from it
// since an AtomicPtr couldn't be dereferenced to a Direction
impl From<usize> for Direction {
    fn from(val: usize) -> Self {
        match val {
            0 => Direction::UP,
            1 => Direction::DOWN,
            2 => Direction::LEFT,
            3 => Direction::RIGHT,
            4 => Direction::STOP,
            _ => unreachable!()
        }
    }
}