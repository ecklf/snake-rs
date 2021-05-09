extern crate chrono;
extern crate rand;
extern crate termion;

use chrono::{DateTime, Local};
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::VecDeque;
use std::io::{stdout, Read, Write};
use std::iter;
use std::thread::sleep;
use std::time::Duration;
use termion::event::{parse_event, Event, Key};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::{async_stdin, clear, color, cursor, style};

// config
pub const START_INTERVAL: u64 = 100;
pub const MIN_INTERVAL: u64 = 75;
pub const INTERVAL_DELTA: u64 = 2;

mod graphics {
    pub const STARTUP_SCREEN: &'static str = "      ___           ___           ___           ___           ___           ___           ___     \r
     /\\__\\         /\\  \\         /\\  \\         /|  |         /\\__\\         /\\  \\         /\\__\\    \r
    /:/ _/_        \\:\\  \\       /::\\  \\       |:|  |        /:/ _/_       /::\\  \\       /:/ _/_   \r
   /:/ /\\  \\        \\:\\  \\     /:/\\:\\  \\      |:|  |       /:/ /\\__\\     /:/\\:\\__\\     /:/ /\\  \\  \r
  /:/ /::\\  \\   _____\\:\\  \\   /:/ /::\\  \\   __|:|  |      /:/ /:/ _/_   /:/ /:/  /    /:/ /::\\  \\ \r
 /:/_/:/\\:\\__\\ /::::::::\\__\\ /:/_/:/\\:\\__\\ /\\ |:|__|____ /:/_/:/ /\\__\\ /:/_/:/__/___ /:/_/:/\\:\\__\\\r
 \\:\\/:/ /:/  / \\:\\~~\\~~\\/__/ \\:\\/:/  \\/__/ \\:\\/:::::/__/ \\:\\/:/ /:/  / \\:\\/:::::/  / \\:\\/:/ /:/  /\r
  \\::/ /:/  /   \\:\\  \\        \\::/__/       \\::/~~/~      \\::/_/:/  /   \\::/~~/~~~~   \\::/ /:/  / \r
   \\/_/:/  /     \\:\\  \\        \\:\\  \\        \\:\\~~\\        \\:\\/:/  /     \\:\\~~\\        \\/_/:/  /  \r
     /:/  /       \\:\\__\\        \\:\\__\\        \\:\\__\\        \\::/  /       \\:\\__\\         /:/  /   \r
     \\/__/         \\/__/         \\/__/         \\/__/         \\/__/         \\/__/         \\/__/    \r
    ";
    pub const TOP_LEFT_CORNER: &'static str = "╔";
    pub const TOP_RIGHT_CORNER: &'static str = "╗";
    pub const BOTTOM_LEFT_CORNER: &'static str = "╚";
    pub const BOTTOM_RIGHT_CORNER: &'static str = "╝";
    pub const VERTICAL_LINE: &'static str = "║";
    pub const HORIZONTAL_LINE: &'static str = "═";
    pub const SNAKE_FRAGMENT: &'static str = "@";
}

use self::graphics::*;

struct Game<R, W: Write> {
    stdin: R,
    stdout: W,
    rng: ThreadRng,
    // game state
    interval: u64,
    width: u16,
    height: u16,
    score: i32,
    snake: Snake,
    munchie: Munchie,
}

#[derive(Copy, Clone)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Copy, Clone, PartialEq)]
struct Position {
    x: u16,
    y: u16,
}

struct Snake {
    direction: Direction,
    fragments: VecDeque<SnakeFragment>,
}

impl Snake {
    fn update(&mut self, has_eaten: bool) {
        let Position { x: h_x, y: h_y } = self.fragments[0].position;

        let new_head = {
            match self.direction {
                Direction::Up => SnakeFragment {
                    position: Position { x: h_x, y: h_y - 1 },
                },
                Direction::Down => SnakeFragment {
                    position: Position { x: h_x, y: h_y + 1 },
                },
                Direction::Left => SnakeFragment {
                    position: Position { x: h_x - 1, y: h_y },
                },
                Direction::Right => SnakeFragment {
                    position: Position { x: h_x + 1, y: h_y },
                },
            }
        };

        self.fragments.push_front(new_head);

        if !has_eaten {
            self.fragments.pop_back().unwrap();
        }
    }

    fn turn(&mut self, direction: Direction) {
        match (self.direction, direction) {
            (Direction::Up, Direction::Down)
            | (Direction::Down, Direction::Up)
            | (Direction::Left, Direction::Right)
            | (Direction::Right, Direction::Left) => return,
            _ => self.direction = direction,
        }
    }
}

#[derive(Copy, Clone)]
struct SnakeFragment {
    position: Position,
}

struct Munchie {
    position: Position,
}

impl<R: Read, W: Write> Game<R, W> {
    fn new(stdin: R, stdout: W) -> Game<R, RawTerminal<W>> {
        Game {
            stdout: stdout.into_raw_mode().unwrap(),
            stdin,
            // game state
            interval: START_INTERVAL,
            width: 80,
            height: 36,
            score: 0,
            rng: rand::thread_rng(),
            snake: Snake {
                direction: Direction::Left,
                fragments: VecDeque::from(vec![
                    SnakeFragment {
                        position: Position { x: 40, y: 20 },
                    },
                    SnakeFragment {
                        position: Position { x: 41, y: 20 },
                    },
                    SnakeFragment {
                        position: Position { x: 42, y: 20 },
                    },
                    SnakeFragment {
                        position: Position { x: 43, y: 20 },
                    },
                    SnakeFragment {
                        position: Position { x: 44, y: 20 },
                    },
                    SnakeFragment {
                        position: Position { x: 45, y: 20 },
                    },
                ]),
            },
            munchie: Munchie {
                position: Position { x: 20, y: 20 },
            },
        }
    }

    fn init(&mut self) {
        write!(
            self.stdout,
            "{}{}{}{}",
            clear::All,
            style::Reset,
            cursor::Hide,
            cursor::Goto(1, 1)
        )
        .unwrap();
        write!(self.stdout, "{}{startup}{}\r\nBy https://github.com/impulse\r\n\nPress {bold}SPACE{reset} to start, {bold}Q{reset} to quit", color::Fg(color::Red), style::Reset, bold = style::Bold, reset = style::Reset, startup = STARTUP_SCREEN).unwrap();
        self.stdout.flush().unwrap();
        loop {
            let mut b = [0];
            self.stdin.read(&mut b).unwrap();
            if b[0] == b' ' {
                self.reset();
                self.game_loop();
                return;
            }
            if b[0] == b'q' {
                return;
            }
        }
    }

    fn pause(&mut self) -> bool {
        let paused_str = String::from("===PAUSED===");

        write!(
            self.stdout,
            "{}{}{}{}{paused}",
            style::Bold,
            color::Bg(color::LightWhite),
            color::Fg(color::Black),
            cursor::Goto(
                (self.width / 2) - (paused_str.len() as u16 / 2),
                self.height / 2
            ),
            paused = paused_str
        )
        .unwrap();
        write!(self.stdout, "{}", style::Reset).unwrap();
        self.stdout.flush().unwrap();

        loop {
            let mut b = [0];
            self.stdin.read(&mut b).unwrap();
            if b[0] == b'p' {
                self.reset();
                return false;
            }
            if b[0] == b'q' {
                self.reset();
                return true;
            }
        }
    }

    fn game_over(&mut self) -> bool {
        let game_over_str = String::from("===GAME OVER===");
        let prompt_str = String::from("Press r to restart");

        write!(
            self.stdout,
            "{}{}{}",
            style::Bold,
            color::Bg(color::LightWhite),
            color::Fg(color::Black)
        )
        .unwrap();
        write!(
            self.stdout,
            "{}{game_over_str}{}{prompt_str}",
            cursor::Goto(
                (self.width / 2) - (game_over_str.len() as u16 / 2),
                (self.height / 2) - 1
            ),
            cursor::Goto(
                (self.width / 2) - (prompt_str.len() as u16 / 2),
                (self.height / 2) + 1
            ),
            game_over_str = game_over_str,
            prompt_str = prompt_str
        )
        .unwrap();
        write!(self.stdout, "{}", style::Reset).unwrap();
        self.stdout.flush().unwrap();

        loop {
            let mut b = [0];
            self.stdin.read(&mut b).unwrap();
            if b[0] == b'r' {
                self.score = 0;
                self.interval = START_INTERVAL;
                self.snake = self.new_snake();
                self.munchie = Munchie {
                    position: Position { x: 20, y: 20 },
                };
                self.reset();
                return false;
            }
            if b[0] == b'q' {
                self.reset();
                return true;
            }
        }
    }

    fn reset(&mut self) {
        write!(
            self.stdout,
            "{}{}{}",
            cursor::Goto(1, 1),
            clear::All,
            style::Reset
        )
        .unwrap();
        self.draw_grid();
        self.stdout.flush().unwrap();
    }

    fn new_snake(&mut self) -> Snake {
        Snake {
            direction: Direction::Left,
            fragments: VecDeque::from(vec![
                SnakeFragment {
                    position: Position { x: 40, y: 20 },
                },
                SnakeFragment {
                    position: Position { x: 41, y: 20 },
                },
                SnakeFragment {
                    position: Position { x: 42, y: 20 },
                },
                SnakeFragment {
                    position: Position { x: 43, y: 20 },
                },
                SnakeFragment {
                    position: Position { x: 44, y: 20 },
                },
                SnakeFragment {
                    position: Position { x: 45, y: 20 },
                },
            ]),
        }
    }

    fn clear_snake(&mut self) {
        for f in &self.snake.fragments {
            let Position { x, y } = f.position;
            write!(self.stdout, "{} ", cursor::Goto(x, y)).unwrap();
        }
    }

    fn draw_snake(&mut self) {
        write!(self.stdout, "{}{}", style::Bold, color::Fg(color::Green)).unwrap();
        for f in &self.snake.fragments {
            let Position { x, y } = f.position;
            write!(self.stdout, "{}{}", cursor::Goto(x, y), SNAKE_FRAGMENT).unwrap();
        }
        write!(self.stdout, "{}", style::Reset).unwrap();
    }

    fn check_munchie(&mut self) -> bool {
        let head_pos = self.snake.fragments[0].position;
        let munchie_pos = self.munchie.position;

        // generate new munchie position when eaten
        if head_pos == munchie_pos {
            write!(
                self.stdout,
                "{} ",
                cursor::Goto(munchie_pos.x, munchie_pos.y)
            )
            .unwrap();

            loop {
                let rng_position = Position {
                    x: self.rng.gen_range(2..self.width),
                    y: self.rng.gen_range(2..self.height),
                };

                if self
                    .snake
                    .fragments
                    .iter()
                    .filter(|fragment| fragment.position == rng_position)
                    .next()
                    .is_some()
                {
                    continue;
                } else {
                    self.munchie.position = rng_position;
                    break;
                }
            }

            write!(
                self.stdout,
                "{}#",
                cursor::Goto(munchie_pos.x, munchie_pos.y)
            )
            .unwrap();
            return true;
        }

        // draw munchie
        write!(self.stdout, "{}{}", style::Bold, color::Fg(color::Red)).unwrap();
        write!(
            self.stdout,
            "{}¤",
            cursor::Goto(munchie_pos.x, munchie_pos.y)
        )
        .unwrap();
        write!(self.stdout, "{}", style::Reset).unwrap();
        return false;
    }

    fn check_collision(&mut self) -> bool {
        let head_pos = self.snake.fragments[0].position;

        // check frame collision and overwrite border element when hit
        if head_pos.x == 1 || head_pos.x == self.width {
            write!(
                self.stdout,
                "{}{}",
                cursor::Goto(head_pos.x, head_pos.y),
                VERTICAL_LINE
            )
            .unwrap();
            return true;
        }

        if head_pos.y == 1 || head_pos.y == self.height {
            write!(
                self.stdout,
                "{}{}",
                cursor::Goto(head_pos.x, head_pos.y),
                HORIZONTAL_LINE
            )
            .unwrap();
            return true;
        }

        // check snake head collision
        for (i, f) in self.snake.fragments.iter().enumerate() {
            if i != 0 && head_pos == f.position {
                return true;
            }
        }

        false
    }

    fn draw_grid(&mut self) {
        // draw corners
        write!(self.stdout, "{}{}", cursor::Goto(1, 1), TOP_LEFT_CORNER).unwrap();
        write!(
            self.stdout,
            "{}{}",
            cursor::Goto(self.width, 1),
            TOP_RIGHT_CORNER
        )
        .unwrap();
        write!(
            self.stdout,
            "{}{}",
            cursor::Goto(1, self.height),
            BOTTOM_LEFT_CORNER
        )
        .unwrap();
        write!(
            self.stdout,
            "{}{}",
            cursor::Goto(self.width, self.height),
            BOTTOM_RIGHT_CORNER
        )
        .unwrap();

        // draw horizontal borders (ignore first and last element)
        for i in 2..self.width {
            write!(self.stdout, "{}{}", cursor::Goto(i, 1), HORIZONTAL_LINE).unwrap();
            write!(
                self.stdout,
                "{}{}",
                cursor::Goto(i, self.height),
                HORIZONTAL_LINE
            )
            .unwrap();
        }

        // draw vertical borders (ignore first and last element)
        for i in 2..self.height {
            write!(self.stdout, "{}{}", cursor::Goto(1, i), VERTICAL_LINE).unwrap();
            write!(
                self.stdout,
                "{}{}",
                cursor::Goto(self.width, i),
                VERTICAL_LINE
            )
            .unwrap();
        }

        self.stdout.flush().unwrap();
    }

    fn game_loop(&mut self) {
        'game_loop: loop {
            // handle user input
            let mut key_bytes = [0];
            self.stdin.read(&mut key_bytes).unwrap();
            if let Ok(event) = parse_event(key_bytes[0], &mut iter::empty::<_>()) {
                match event {
                    Event::Key(Key::Char('q')) => break 'game_loop,
                    Event::Key(Key::Char('p')) => {
                        if self.pause() == true {
                            break 'game_loop;
                        }
                    }
                    Event::Key(Key::Char('k')) => self.snake.turn(Direction::Up),
                    Event::Key(Key::Char('j')) => self.snake.turn(Direction::Down),
                    Event::Key(Key::Char('h')) => self.snake.turn(Direction::Left),
                    Event::Key(Key::Char('l')) => self.snake.turn(Direction::Right),
                    _ => (),
                }
            }

            // run game events
            let has_eaten = self.check_munchie();
            if has_eaten {
                self.score += 1;
                if self.interval >= MIN_INTERVAL {
                    self.interval -= INTERVAL_DELTA;
                }
            }

            // update snake
            self.clear_snake();
            self.snake.update(has_eaten);
            self.draw_snake();

            // determine game state
            if self.check_collision() {
                if self.game_over() {
                    break 'game_loop;
                }
            }

            // display stats
            let current_time = self.get_local_time();
            write!(self.stdout, "{}{}{}{}{}", style::Bold, color::Bg(color::LightWhite), color::Fg(color::Black), cursor::Goto(1, self.height + 1), format!("= Score: {score} | Controls: up/down/left/right: k/j/h/l / Pause: p | Time: {time} =", score = self.score.to_string(), time = current_time)).unwrap();
            write!(self.stdout, "{}", style::Reset).unwrap();
            self.stdout.flush().unwrap();

            // next iteration
            sleep(Duration::from_millis(self.interval));
        }
    }

    fn get_local_time(&mut self) -> String {
        let now: DateTime<Local> = Local::now();
        now.format("%H:%M:%S").to_string()
    }
}

fn main() {
    let stdout = stdout();
    let mut app = Game::new(async_stdin(), stdout.lock());
    app.init();
    // clear the screen and show cursor when exiting
    write!(
        app.stdout,
        "{}{}{}{}",
        clear::All,
        style::Reset,
        cursor::Show,
        cursor::Goto(1, 1)
    )
    .unwrap();
    app.stdout.flush().unwrap();
}
