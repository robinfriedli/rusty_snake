use crate::{FIELD_HEIGHT, FIELD_WIDTH};
use crate::direction::Direction;

pub struct Snake {
    pub(crate) x_pos: u16,
    pub(crate) y_pos: u16,
    pub(crate) tail_x_pos: Vec<u16>,
    pub(crate) tail_y_pos: Vec<u16>,
}

impl Snake {
    pub fn new() -> Snake {
        Snake {
            // spawn head in the middle of the field
            x_pos: FIELD_WIDTH / 2,
            y_pos: FIELD_HEIGHT / 2,
            tail_x_pos: Vec::new(),
            tail_y_pos: Vec::new(),
        }
    }

    pub fn move_pos(&mut self, direction: &Direction) {
        match direction {
            Direction::UP => {
                self.move_tail();
                self.y_pos -= 1;
            }
            Direction::DOWN => {
                self.move_tail();
                self.y_pos += 1;
            }
            Direction::LEFT => {
                self.move_tail();
                self.x_pos -= 1;
            }
            Direction::RIGHT => {
                self.move_tail();
                self.x_pos += 1
            }
            Direction::STOP => {}
        };
    }

    pub fn move_tail(&mut self) {
        if self.tail_x_pos.len() == 0 {
            return;
        }

        // iterate backwards to update last element first since each element depends on the old value
        // of the element in front of it
        for i in (0..self.tail_x_pos.len()).rev() {
            if i == 0 {
                self.tail_x_pos[0] = self.x_pos;
                self.tail_y_pos[0] = self.y_pos;
            } else {
                self.tail_x_pos[i] = self.tail_x_pos[i - 1];
                self.tail_y_pos[i] = self.tail_y_pos[i - 1];
            }
        }
    }

    // spawn new element outside of view
    pub fn append_tail(&mut self) {
        self.tail_x_pos.push(FIELD_HEIGHT + 1);
        self.tail_y_pos.push(FIELD_WIDTH + 1);
    }

    pub fn create_tail_matrix(&self) -> Vec<Vec<bool>> {
        let mut matrix: Vec<Vec<bool>> = Vec::with_capacity(FIELD_HEIGHT as usize);

        for _y in 0..FIELD_HEIGHT {
            let mut row: Vec<bool> = Vec::with_capacity(FIELD_WIDTH as usize);

            for _x in 0..FIELD_WIDTH {
                row.push(false);
            }

            matrix.push(row);
        }

        for i in 0..self.tail_x_pos.len() {
            let curr_x = self.tail_x_pos[i];
            let curr_y = self.tail_y_pos[i];

            // mind that newly created tail elements are spawned out of view
            if curr_x < FIELD_WIDTH && curr_y < FIELD_HEIGHT {
                matrix[curr_y as usize][curr_x as usize] = true;
            }
        }

        return matrix;
    }

    pub fn reset(&mut self) {
        self.x_pos = FIELD_WIDTH / 2;
        self.y_pos = FIELD_HEIGHT / 2;
        self.tail_x_pos.clear();
        self.tail_y_pos.clear();
    }
}