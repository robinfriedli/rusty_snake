use std::fmt;
use std::convert::TryFrom;

#[derive(Debug)]
pub enum Difficulty {
    EASY,
    ARCADE,
    NORMAL,
    HARD,
}

impl fmt::Display for Difficulty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<u32> for Difficulty {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Difficulty::EASY),
            1 => Ok(Difficulty::ARCADE),
            2 => Ok(Difficulty::NORMAL),
            3 => Ok(Difficulty::HARD),
            _ => Err(())
        }
    }
}

impl Difficulty {
    pub(crate) fn is_game_over_on_wall_collision(&self) -> bool {
        match self {
            Difficulty::EASY => false,
            Difficulty::ARCADE => false,
            Difficulty::NORMAL => true,
            Difficulty::HARD => true
        }
    }

    pub(crate) fn get_refresh_delay(&self) -> u64 {
        match self {
            Difficulty::EASY => 150,
            Difficulty::ARCADE => 50,
            Difficulty::NORMAL => 150,
            Difficulty::HARD => 50
        }
    }

    pub(crate) fn get_description(&self) -> &str {
        match self {
            Difficulty::EASY => "No game over on wall collision and slow speed",
            Difficulty::ARCADE => "Easy but speedy",
            Difficulty::NORMAL => "Game over on wall collision, normal speed",
            Difficulty::HARD => "Dangerous walls and fast speed"
        }
    }

    pub(crate) fn get_score_multiplier(&self) -> u16 {
        match self {
            Difficulty::EASY => 1,
            Difficulty::ARCADE => 2,
            Difficulty::NORMAL => 3,
            Difficulty::HARD => 5
        }
    }
}