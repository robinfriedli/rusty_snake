extern crate ncurses;

use core::{fmt, ptr};
use std::convert::{TryFrom, TryInto};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write, Read};
use std::sync::atomic::{AtomicUsize, Ordering, AtomicBool};
use std::{thread, io};
use std::thread::Builder;
use std::time::Duration;
use ncurses::*;

use rand::Rng;
use crate::score_manager::ScoreManager;
use std::ops::Add;
use futures::future::Future;
use completable_future::CompletableFuture;
use std::cell::{RefCell, UnsafeCell};

mod score_manager;

#[derive(Debug)]
enum Direction {
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
    fn is_game_over_on_wall_collision(&self) -> bool {
        match self {
            Difficulty::EASY => false,
            Difficulty::ARCADE => false,
            Difficulty::NORMAL => true,
            Difficulty::HARD => true
        }
    }

    fn get_refresh_delay(&self) -> u64 {
        match self {
            Difficulty::EASY => 150,
            Difficulty::ARCADE => 50,
            Difficulty::NORMAL => 150,
            Difficulty::HARD => 50
        }
    }

    fn get_description(&self) -> &str {
        match self {
            Difficulty::EASY => "No game over on wall collision and slow speed",
            Difficulty::ARCADE => "Easy but speedy",
            Difficulty::NORMAL => "Game over on wall collision, normal speed",
            Difficulty::HARD => "Dangerous walls and fast speed"
        }
    }

    fn get_score_multiplier(&self) -> u16 {
        match self {
            Difficulty::EASY => 1,
            Difficulty::ARCADE => 2,
            Difficulty::NORMAL => 3,
            Difficulty::HARD => 5
        }
    }
}

struct Fruit {
    x_pos: u16,
    y_pos: u16,
}

struct Snake {
    x_pos: u16,
    y_pos: u16,
    tail_x_pos: Vec<u16>,
    tail_y_pos: Vec<u16>,
}

impl Snake {
    fn new() -> Snake {
        Snake {
            // spawn head in the middle of the field
            x_pos: FIELD_WIDTH / 2,
            y_pos: FIELD_HEIGHT / 2,
            tail_x_pos: Vec::new(),
            tail_y_pos: Vec::new(),
        }
    }

    fn move_pos(&mut self, direction: Direction) {
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

    fn move_tail(&mut self) {
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
    fn append_tail(&mut self) {
        self.tail_x_pos.push(FIELD_HEIGHT + 1);
        self.tail_y_pos.push(FIELD_WIDTH + 1);
    }

    fn create_tail_matrix(&self) -> Vec<Vec<bool>> {
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

    fn reset(&mut self) {
        self.x_pos = FIELD_WIDTH / 2;
        self.y_pos = FIELD_HEIGHT / 2;
        self.tail_x_pos.clear();
        self.tail_y_pos.clear();
    }
}

impl Fruit {
    fn new() -> Fruit {
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

    fn respawn(&mut self) {
        let rand_location = Self::generate_rand_location();
        self.x_pos = rand_location.0;
        self.y_pos = rand_location.1;
    }
}

struct FutureHolder {
    future: UnsafeCell<Option<CompletableFuture<i32, ()>>>
}

impl FutureHolder {
    const fn new() -> FutureHolder {
        FutureHolder { future: UnsafeCell::new(None) }
    }

    fn prepare(&self) {
        unsafe {
            self.future.get().replace(Some(CompletableFuture::<i32, ()>::new()));
        }
    }

    /// Replaces the current Option with None and transfers ownership of the previous value to the caller
    fn consume(&self) -> Option<CompletableFuture<i32, ()>> {
        unsafe {
            self.future.get().replace(None)
        }
    }

    /// wait for the value to be completed in another thread, blocking the current thread, this consumes
    /// the current future
    fn wait(&self) -> Option<i32> {
        let consumed_val = self.consume();
        match consumed_val {
            Some(fut) => {
                let val = fut.wait().unwrap();
                return Some(val);
            }
            None => {
                return None;
            }
        };
    }

    fn is_empty(&self) -> bool {
        unsafe {
            self.future.get().read().is_none()
        }
    }

    fn complete_optional(&self, val: i32) {
        unsafe {
            match self.future.get().read() {
                Some(fut) => {
                    fut.signal().complete(val);
                }
                None => {}
            };
        }
    }
}

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        endwin();
    }
}

const WALL_SYMBOL: char = '#';
const FRUIT_SYMBOL: char = 'F';
const HEAD_SYMBOL: char = 'O';
const TAIL_SYMBOL: char = 'o';

const FIELD_WIDTH: u16 = 50;
const FIELD_HEIGHT: u16 = 50;

const UP_KEY: i32 = 'w' as i32;
const DOWN_KEY: i32 = 's' as i32;
const LEFT_KEY: i32 = 'a' as i32;
const RIGHT_KEY: i32 = 'd' as i32;
const PAUSE_KEY: i32 = 'p' as i32;
const RETRY_KEY: i32 = 'r' as i32;
const QUIT_KEY: i32 = 'q' as i32;

static DIRECTION: AtomicUsize = AtomicUsize::new(Direction::STOP as usize);
static LISTENING_FOR_GAME_TERMINATION: AtomicBool = AtomicBool::new(false);
static GAME_TERMINATED: AtomicBool = AtomicBool::new(false);
static GAME_RETRY: AtomicBool = AtomicBool::new(false);

fn main() {
    // make sure endwin() is called even on panic
    let _cleanup = Cleanup;
    let mut score_manager = ScoreManager::from_file("scores.xml");

    print!("\x1B[2J");
    print_start_screen();
    let mut user_name_temp = String::new();
    io::stdin().read_line(&mut user_name_temp).expect("could not read user name");
    let user_name = user_name_temp.trim();
    initscr();
    let difficulty = select_difficulty();
    let mut snake = Snake::new();
    let mut fruit = Fruit::new();

    let mut game_over = false;
    let mut game_quit = false;
    let mut current_score: u64 = 0;
    let high_scores = score_manager.get_high_scores(&difficulty, 1);
    let mut high_score_display = create_high_score_display(&high_scores);

    let _x = Builder::new().name(String::from("input-handler")).spawn(|| {
        loop {
            let input = getch();

            if LISTENING_FOR_GAME_TERMINATION.load(Ordering::Relaxed) {
                if input == RETRY_KEY {
                    GAME_RETRY.store(true, Ordering::Relaxed);
                } else if input == QUIT_KEY {
                    GAME_TERMINATED.store(true, Ordering::Relaxed);
                }
            } else {
                if input == UP_KEY {
                    DIRECTION.store(Direction::UP as usize, Ordering::Relaxed);
                } else if input == DOWN_KEY {
                    DIRECTION.store(Direction::DOWN as usize, Ordering::Relaxed);
                } else if input == LEFT_KEY {
                    DIRECTION.store(Direction::LEFT as usize, Ordering::Relaxed);
                } else if input == RIGHT_KEY {
                    DIRECTION.store(Direction::RIGHT as usize, Ordering::Relaxed);
                } else if input == PAUSE_KEY {
                    DIRECTION.store(Direction::STOP as usize, Ordering::Relaxed);
                }
            }

            refresh();
        }
    });

    while !GAME_TERMINATED.load(Ordering::Relaxed) {
        while !game_over {
            clear();
            draw(&snake, &fruit, current_score, &high_score_display, &difficulty);

            refresh();

            snake.move_pos(DIRECTION.load(Ordering::Relaxed).into());

            // do not use an else if here since moving the snake when is_game_over_on_wall_collision is
            // false might result in the snake being placed an a fruit, so this should always get checked
            if snake.x_pos == 0 || snake.x_pos == FIELD_WIDTH - 1 || snake.y_pos == 0 || snake.y_pos == FIELD_HEIGHT - 1 {
                if difficulty.is_game_over_on_wall_collision() {
                    game_over = true;
                } else {
                    if snake.x_pos == 0 {
                        snake.x_pos = FIELD_WIDTH - 2;
                    } else if snake.x_pos == FIELD_WIDTH - 1 {
                        snake.x_pos = 1;
                    }
                    if snake.y_pos == 0 {
                        snake.y_pos = FIELD_HEIGHT - 2;
                    } else if snake.y_pos == FIELD_HEIGHT - 1 {
                        snake.y_pos = 1;
                    }
                }
            }

            for i in 0..snake.tail_x_pos.len() {
                if snake.tail_x_pos[i] == snake.x_pos && snake.tail_y_pos[i] == snake.y_pos {
                    game_over = true;
                }
            }

            if snake.x_pos == fruit.x_pos && snake.y_pos == fruit.y_pos {
                current_score += 5 * difficulty.get_score_multiplier() as u64;
                snake.append_tail();
                fruit.respawn();
            }

            thread::sleep(Duration::from_millis(difficulty.get_refresh_delay()));
        }

        score_manager.write_score(current_score, &difficulty, user_name);
        let new_high_scores = score_manager.get_high_scores(&difficulty, 3);
        print_game_over_screen(current_score, &new_high_scores, &difficulty);
        LISTENING_FOR_GAME_TERMINATION.store(true, Ordering::Relaxed);
        loop {
            if GAME_RETRY.load(Ordering::Relaxed) {
                high_score_display = create_high_score_display(&new_high_scores);
                current_score = 0;
                fruit.respawn();
                snake.reset();
                DIRECTION.store(Direction::STOP as usize, Ordering::Relaxed);
                game_over = false;
                LISTENING_FOR_GAME_TERMINATION.store(false, Ordering::Relaxed);
                GAME_RETRY.store(false, Ordering::Relaxed);
                break;
            } else if GAME_TERMINATED.load(Ordering::Relaxed) {
                break;
            }

            thread::sleep(Duration::from_millis(50));
        }
    }
}

fn draw(snake: &Snake, fruit: &Fruit, score: u64, high_score_display: &String, difficulty: &Difficulty) {
    let max_y_index = FIELD_HEIGHT - 1;
    let max_x_index = FIELD_WIDTH - 1;
    let tail_matrix = snake.create_tail_matrix();

    for y in 0..FIELD_HEIGHT {
        for x in 0..FIELD_WIDTH {
            if y == 0 || x == 0 || y == max_y_index || x == max_x_index {
                addch(WALL_SYMBOL as u32);
            } else if y == snake.y_pos && x == snake.x_pos {
                addch(HEAD_SYMBOL as u32);
            } else if y == fruit.y_pos && x == fruit.x_pos {
                addch(FRUIT_SYMBOL as u32);
            } else if tail_matrix[y as usize][x as usize] {
                addch(TAIL_SYMBOL as u32);
            } else {
                addch(' ' as u32);
            }
        }

        addch('\n' as u32);
    }
    addch('\n' as u32);
    addstr(format!("Score:                                  {}", score).as_str());
    addch('\n' as u32);
    addstr(format!("High score (for current difficulty):    {}", high_score_display).as_str());
    addch('\n' as u32);
    addstr(format!("Tail length:                            {}", snake.tail_x_pos.len()).as_str());
    addch('\n' as u32);
    addstr(format!("Head pos:                               x: {}\n                                        y: {}", snake.x_pos, snake.y_pos).as_str());
    addch('\n' as u32);
    let direction: Direction = DIRECTION.load(Ordering::Relaxed).into();
    addstr(format!("Direction:                              {}", direction).as_str());
    addch('\n' as u32);
    addstr(format!("Difficulty:                             {}", difficulty).as_str());
}

fn select_difficulty() -> Difficulty {
    let difficulty;
    loop {
        print_difficulty_selection();
        let selected: Result<u8, _> = getch().try_into();

        match selected {
            Ok(selected_u8) => {
                let selected_char = selected_u8 as char;
                let digit_conversion = selected_char.to_digit(10);
                match digit_conversion {
                    Some(digit) => {
                        let difficulty_conversion: Result<Difficulty, _> = digit.try_into();
                        match difficulty_conversion {
                            Ok(selected_difficulty) => {
                                difficulty = selected_difficulty;
                                break;
                            }
                            Err(_) => {
                                clear();
                                addstr(format!("Could not get difficulty for {}", digit).as_str());
                                addch('\n' as u32);
                                refresh();
                                continue;
                            }
                        }
                    }
                    None => {
                        clear();
                        addstr("Could not convert char to digit");
                        addch('\n' as u32);
                        refresh();
                        continue;
                    }
                }
            }
            Err(_) => {
                clear();
                addstr("Could not convert input to ascii");
                addch('\n' as u32);
                refresh();
                continue;
            }
        }
    }
    addch('\n' as u32);
    addstr(format!("Selected difficulty {}", difficulty).as_str());
    refresh();

    return difficulty;
}

fn print_difficulty_selection() {
    addstr("Select difficulty:");
    addch('\n' as u32);
    addstr(format!("{} - {}: {}", Difficulty::EASY as u8, Difficulty::EASY, Difficulty::EASY.get_description()).as_str());
    addch('\n' as u32);
    addstr(format!("{} - {}: {}", Difficulty::ARCADE as u8, Difficulty::ARCADE, Difficulty::ARCADE.get_description()).as_str());
    addch('\n' as u32);
    addstr(format!("{} - {}: {}", Difficulty::NORMAL as u8, Difficulty::NORMAL, Difficulty::NORMAL.get_description()).as_str());
    addch('\n' as u32);
    addstr(format!("{} - {}: {}", Difficulty::HARD as u8, Difficulty::HARD, Difficulty::HARD.get_description()).as_str());
    refresh();
}

fn create_high_score_display(high_score_vec: &Vec<(u64, String)>) -> String {
    if high_score_vec.is_empty() {
        String::from("0")
    } else {
        let high_score_tuple = &high_score_vec[0];
        String::from(high_score_tuple.0.to_string()).add(" (").add(high_score_tuple.1.as_str()).add(")")
    }
}

fn print_start_screen() {
    let start_text = r#"
 ______   ___   __    ________   ___   ___   ______
/_____/\ /__/\ /__/\ /_______/\ /___/\/__/\ /_____/\
\::::_\/_\::\_\\  \ \\::: _  \ \\::.\ \\ \ \\::::_\/_
 \:\/___/\\:. `-\  \ \\::(_)  \ \\:: \/_) \ \\:\/___/\
  \_::._\:\\:. _    \ \\:: __  \ \\:. __  ( ( \::___\/_
    /____\:\\. \`-\  \ \\:.\ \  \ \\: \ )  \ \ \:\____/\
    \_____\/ \__\/ \__\/ \__\/\__\/ \__\/\__\/  \_____\/

Controls:
W:  UP
S:  DOWN
A:  LEFT
D:  RIGHT
P:  PAUSE (click any other direction to unpause)
___________________
Enter player name:
-------------------
    "#;
    println!("{}", start_text);
}

fn print_game_over_screen(current_score: u64, high_scores: &Vec<(u64, String)>, difficulty: &Difficulty) {
    clear();
    refresh();
    let game_over_text = r#"
 _____   ___  ___  ___ _____   _____  _   _ ___________
|  __ \ / _ \ |  \/  ||  ___| |  _  || | | |  ___| ___ \
| |  \// /_\ \| .  . || |__   | | | || | | | |__ | |_/ /
| | __ |  _  || |\/| ||  __|  | | | || | | |  __||    /
| |_\ \| | | || |  | || |___  \ \_/ /\ \_/ / |___| |\ \
 \____/\_| |_/\_|  |_/\____/   \___/  \___/\____/\_| \_|


    "#;

    let mut output = String::from(game_over_text).add("Your score:\n")
        .add(current_score.to_string().as_str()).add("\n\n")
        .add("High scores (").add(difficulty.to_string().as_str()).add(")\n");

    for score_tuple in high_scores {
        let line = String::from(score_tuple.1.as_str()).add(":\t").add(score_tuple.0.to_string().as_str()).add("\n");
        output.push_str(line.as_str());
    }

    output.push_str("\n\nPress r to retry or q to quit.");
    addstr(output.as_str());
    refresh();
}