use rand::Rng;
use crate::{FIELD_WIDTH, FIELD_HEIGHT};

pub struct Fruit {
    pub(crate) x_pos: u16,
    pub(crate) y_pos: u16,
}

impl Fruit {
    pub(crate) fn new() -> Fruit {
        let rand_location = Self::generate_rand_location();

        Fruit {
            x_pos: rand_location.0,
            y_pos: rand_location.1,
        }
    }

    fn generate_rand_location() -> (u16, u16) {
        let mut rng = rand::thread_rng();

        let pos_x = rng.gen_range(1, FIELD_WIDTH - 1);
        let pos_y = rng.gen_range(1, FIELD_HEIGHT - 1);

        (pos_x, pos_y)
    }

    pub(crate) fn respawn(&mut self) {
        let rand_location = Self::generate_rand_location();
        self.x_pos = rand_location.0;
        self.y_pos = rand_location.1;
    }
}