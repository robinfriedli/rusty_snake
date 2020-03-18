use core::fmt;
use std::{io, thread};
use std::convert::TryInto;
use std::ops::Add;
use std::time::Duration;

use pancurses::*;
use stopwatch::Stopwatch;

use crate::difficulty::Difficulty;
use crate::direction::Direction;
use crate::direction::Direction::STOP;
use crate::duration_formatter::DurationFormatter;
use crate::fruit::Fruit;
use crate::score_manager::ScoreManager;
use crate::snake::Snake;

mod difficulty;
mod direction;
mod duration_formatter;
mod fruit;
mod score_manager;
mod snake;

const WALL_SYMBOL: char = '#';
const FRUIT_SYMBOL: char = 'F';
const HEAD_SYMBOL: char = 'O';
const TAIL_SYMBOL: char = 'o';

const FIELD_WIDTH: u16 = 50;
const FIELD_HEIGHT: u16 = 50;

const UP_KEY: char = 'w';
const DOWN_KEY: char = 's';
const LEFT_KEY: char = 'a';
const RIGHT_KEY: char = 'd';
const PAUSE_KEY: char = 'p';
const RETRY_KEY: char = 'r';
const QUIT_KEY: char = 'q';

struct GameState {
    game_over: bool,
    game_terminated: bool,
    current_direction: Direction,
    current_score: u64,
}

impl GameState {
    fn new() -> Self {
        GameState {
            game_over: false,
            game_terminated: false,
            current_direction: Direction::STOP,
            current_score: 0,
        }
    }
}

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        endwin();
    }
}

fn main() {
    // make sure endwin() is called even on panic
    let _cleanup = Cleanup;
    let score_manager = ScoreManager::from_file("scores.xml");

    print!("\x1B[2J");
    print_start_screen(score_manager.get_total_playtime_display());
    let mut user_name_temp = String::new();
    io::stdin().read_line(&mut user_name_temp).expect("could not read user name");
    let user_name = user_name_temp.trim();

    let window = initscr();
    noecho();
    let rows = FIELD_HEIGHT as i32 + 20;
    let cols = FIELD_WIDTH as i32 + 20;
    if window.get_max_x() < cols || window.get_max_y() < rows {
        resize_term(rows, cols);
    }

    let difficulty = select_difficulty(&window);
    window.nodelay(true);
    let mut snake = Snake::new();
    let mut fruit = Fruit::new();

    let mut game_state = GameState::new();
    let high_scores = score_manager.get_high_scores(&difficulty, 1);
    let mut high_score_display = create_high_score_display(&high_scores);
    let mut stopwatch = stopwatch::Stopwatch::new();

    while !game_state.game_terminated {
        while !game_state.game_over {
            window.clear();
            draw(&window, &snake, &fruit, &game_state, &high_score_display, &difficulty, &stopwatch);
            window.refresh();
            game_state.current_direction = handle_input(&window, game_state.current_direction);
            handle_stopwatch(&mut stopwatch, &game_state.current_direction);
            handle_snake_movement(&mut snake, &mut fruit, &difficulty, &mut game_state);

            thread::sleep(Duration::from_millis(difficulty.get_refresh_delay()));
        }

        score_manager.write_score(game_state.current_score, &difficulty, user_name, stopwatch.elapsed().as_millis());
        let new_high_scores = score_manager.get_high_scores(&difficulty, 3);
        print_game_over_screen(game_state.current_score, &new_high_scores, &difficulty, &stopwatch, &window);

        window.nodelay(false);
        loop {
            match window.getch() {
                Some(Input::Character(RETRY_KEY)) => {
                    high_score_display = create_high_score_display(&new_high_scores);
                    game_state.current_score = 0;
                    game_state.game_over = false;
                    game_state.current_direction = Direction::STOP;
                    fruit.respawn();
                    snake.reset();
                    stopwatch.reset();
                    break;
                }
                Some(Input::Character(QUIT_KEY)) => {
                    game_state.game_terminated = true;
                    break;
                }
                _ => {}
            }
        }
        window.nodelay(true);
    }
}

fn handle_input(window: &Window, curr_dir: Direction) -> Direction {
    match window.getch() {
        Some(Input::Character(UP_KEY)) => Direction::UP,
        Some(Input::Character(DOWN_KEY)) => Direction::DOWN,
        Some(Input::Character(LEFT_KEY)) => Direction::LEFT,
        Some(Input::Character(RIGHT_KEY)) => Direction::RIGHT,
        Some(Input::Character(PAUSE_KEY)) => Direction::STOP,
        _ => curr_dir
    }
}

fn handle_stopwatch(stopwatch: &mut Stopwatch, current_direction: &Direction) {
    if *current_direction == STOP && stopwatch.is_running() {
        stopwatch.stop();
    } else if *current_direction != STOP && !stopwatch.is_running() {
        stopwatch.start();
    }
}

fn handle_snake_movement(snake: &mut Snake, fruit: &mut Fruit, difficulty: &Difficulty, game_state: &mut GameState) {
    snake.move_pos(&game_state.current_direction);
    // do not use an else if here since moving the snake when is_game_over_on_wall_collision is
    // false might result in the snake being placed an a fruit, so this should always get checked
    if snake.x_pos == 0 || snake.x_pos == FIELD_WIDTH - 1 || snake.y_pos == 0 || snake.y_pos == FIELD_HEIGHT - 1 {
        if difficulty.is_game_over_on_wall_collision() {
            game_state.game_over = true
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
            game_state.game_over = true
        }
    }

    if snake.x_pos == fruit.x_pos && snake.y_pos == fruit.y_pos {
        game_state.current_score += 5 * difficulty.get_score_multiplier() as u64;
        snake.append_tail();
        fruit.respawn();
    }
}

fn draw(window: &Window, snake: &Snake, fruit: &Fruit, game_state: &GameState, high_score_display: &String, difficulty: &Difficulty, stopwatch: &Stopwatch) {
    let max_y_index = FIELD_HEIGHT - 1;
    let max_x_index = FIELD_WIDTH - 1;
    let tail_matrix = snake.create_tail_matrix();

    for y in 0..FIELD_HEIGHT {
        for x in 0..FIELD_WIDTH {
            if y == 0 || x == 0 || y == max_y_index || x == max_x_index {
                window.addch(WALL_SYMBOL);
            } else if y == snake.y_pos && x == snake.x_pos {
                window.addch(HEAD_SYMBOL);
            } else if y == fruit.y_pos && x == fruit.x_pos {
                window.addch(FRUIT_SYMBOL);
            } else if tail_matrix[y as usize][x as usize] {
                window.addch(TAIL_SYMBOL);
            } else {
                window.addch(' ');
            }
        }

        window.addch('\n');
    }
    window.addch('\n');
    window.addstr(format!("Score:                                  {}", game_state.current_score).as_str());
    window.addch('\n');
    window.addstr(format!("High score (for current difficulty):    {}", high_score_display).as_str());
    window.addch('\n');
    window.addstr(format!("Tail length:                            {}", snake.tail_x_pos.len()).as_str());
    window.addch('\n');
    window.addstr(format!("Head pos:                               x: {}\n                                        y: {}", snake.x_pos, snake.y_pos).as_str());
    window.addch('\n');
    window.addstr(format!("Direction:                              {}", game_state.current_direction).as_str());
    window.addch('\n');
    window.addstr(format!("Difficulty:                             {}", difficulty).as_str());
    window.addch('\n');
    window.addstr(format!("Duration:                               {}", stopwatch.elapsed().format_duration()).as_str());
}

fn select_difficulty(window: &Window) -> Difficulty {
    loop {
        print_difficulty_selection(window);
        match window.getch() {
            Some(Input::Character(input_char)) => {
                let digit_conversion = input_char.to_digit(10);
                if let Some(digit) = digit_conversion {
                    let difficulty_conversion: Result<Difficulty, _> = digit.try_into();
                    if let Ok(difficulty) = difficulty_conversion {
                        return difficulty;
                    }
                }

                window.clear();
                window.addstr(format!("Could net get difficulty for {}", input_char));
                window.addch('\n');
                window.refresh();
            }
            _ => {}
        }
    }
}

fn print_difficulty_selection(window: &Window) {
    window.addstr("Select difficulty:");
    window.addch('\n');
    window.addstr(format!("{} - {}: {}", Difficulty::EASY as u8, Difficulty::EASY, Difficulty::EASY.get_description()).as_str());
    window.addch('\n');
    window.addstr(format!("{} - {}: {}", Difficulty::ARCADE as u8, Difficulty::ARCADE, Difficulty::ARCADE.get_description()).as_str());
    window.addch('\n');
    window.addstr(format!("{} - {}: {}", Difficulty::NORMAL as u8, Difficulty::NORMAL, Difficulty::NORMAL.get_description()).as_str());
    window.addch('\n');
    window.addstr(format!("{} - {}: {}", Difficulty::HARD as u8, Difficulty::HARD, Difficulty::HARD.get_description()).as_str());
    window.refresh();
}

fn create_high_score_display(high_score_vec: &Vec<(u64, String, Option<u64>)>) -> String {
    if high_score_vec.is_empty() {
        String::from("0")
    } else {
        let high_score_tuple = &high_score_vec[0];
        let time_string = high_score_tuple.2.format_duration();
        String::from(high_score_tuple.0.to_string()).add(" (").add(high_score_tuple.1.as_str()).add(")").add(time_string.as_str())
    }
}

fn print_start_screen(playtime_display: String) {
    println!(r#"
 ______   ___   __    ________   ___   ___   ______
/_____/\ /__/\ /__/\ /_______/\ /___/\/__/\ /_____/\
\::::_\/_\::\_\\  \ \\::: _  \ \\::.\ \\ \ \\::::_\/_
 \:\/___/\\:. `-\  \ \\::(_)  \ \\:: \/_) \ \\:\/___/\
  \_::._\:\\:. _    \ \\:: __  \ \\:. __  ( ( \::___\/_
    /____\:\\. \`-\  \ \\:.\ \  \ \\: \ )  \ \ \:\____/\
    \_____\/ \__\/ \__\/ \__\/\__\/ \__\/\__\/  \_____\/

Total playtime: {}

Controls:
W:  UP
S:  DOWN
A:  LEFT
D:  RIGHT
P:  PAUSE (click any other direction to unpause)
___________________
Enter player name:
-------------------
    "#, playtime_display);
}

fn print_game_over_screen(current_score: u64, high_scores: &Vec<(u64, String, Option<u64>)>, difficulty: &Difficulty, stopwatch: &Stopwatch, window: &Window) {
    window.clear();
    window.refresh();
    let game_over_text = r#"
 _____   ___  ___  ___ _____   _____  _   _ ___________
|  __ \ / _ \ |  \/  ||  ___| |  _  || | | |  ___| ___ \
| |  \// /_\ \| .  . || |__   | | | || | | | |__ | |_/ /
| | __ |  _  || |\/| ||  __|  | | | || | | |  __||    /
| |_\ \| | | || |  | || |___  \ \_/ /\ \_/ / |___| |\ \
 \____/\_| |_/\_|  |_/\____/   \___/  \___/\____/\_| \_|


    "#;

    let mut output = String::from(game_over_text).add("\nYour score:\n")
        .add(current_score.to_string().as_str()).add("\n\n")
        .add("Your time:\n")
        .add(stopwatch.elapsed().format_duration().as_str()).add("\n\n\n")
        .add("High scores (").add(difficulty.to_string().as_str()).add(")\n");

    for score_tuple in high_scores {
        let line = String::from(score_tuple.1.as_str()).add(":\t\t\t").add(score_tuple.0.to_string().as_str()).add(score_tuple.2.format_duration().as_str()).add("\n");
        output.push_str(line.as_str());
    }

    output.push_str("\n\nPress r to retry or q to quit.");
    window.addstr(output.as_str());
    window.refresh();
}