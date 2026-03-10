#![windows_subsystem = "windows"]

use iced::time::{self, seconds};
use iced::widget::{button, center, column, mouse_area, row, text};
use iced::{Center, Color, Element, Length, Subscription, Theme};
use rand::prelude::*;
use std::collections::VecDeque;

const DEFAULT_GRID_SIZE: usize = 15;
const DEFAULT_MINE_COUNT: usize = DEFAULT_GRID_SIZE * 2;
const BLUE: Color = Color::from_rgb(0.0, 0.0, 1.0);
const GREEN: Color = Color::from_rgb(0.0, 0.5, 0.0);
const RED: Color = Color::from_rgb(1.0, 0.0, 0.0);
const DARK_BLUE: Color = Color::from_rgb(0.0, 0.0, 0.5);
const DARK_RED: Color = Color::from_rgb(0.5, 0.0, 0.0);
const TEAL: Color = Color::from_rgb(0.0, 0.5, 0.5);
const BLACK: Color = Color::BLACK;
const GRAY: Color = Color::from_rgb(0.5, 0.5, 0.5);

pub fn main() -> iced::Result {
    iced::application(AppState::default, AppState::update, AppState::view)
        .subscription(AppState::subscription)
        .theme(AppState::theme)
        .run()
}

#[derive(Default)]
enum AppState {
    #[default]
    MainMenu,
    SinglePlayer(Game),
    Multiplayer(MultiplayerState),
}

struct MultiplayerState {
    game: Game,
    // socket: WebSocket,
}

struct Game {
    grid_size: usize,
    grid: Vec<Vec<Cell>>,
    squares_cleared: usize,
    mine_count: usize,
    flags: usize,
    game_over: bool,
    game_won: bool,
    running: bool,
    seconds: u32,
}

impl Default for Game {
    fn default() -> Self {
        let game = Self {
            grid_size: DEFAULT_GRID_SIZE,
            grid: vec![vec![Cell::default(); DEFAULT_GRID_SIZE]; DEFAULT_GRID_SIZE],
            squares_cleared: 0,
            mine_count: DEFAULT_MINE_COUNT,
            flags: 0,
            game_over: false,
            game_won: false,
            running: false,
            seconds: 0,
        };
        game
    }
}

#[derive(Default, Clone, Debug)]
struct Cell {
    is_revealed: bool,
    is_mine: bool,
    is_flaged: bool,
    number: u8,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    MainMenu,
    SinglePlayer,
    Multiplayer,
    Reveal(usize, usize),
    Flag(usize, usize),
    NewGame,
    Tick,
}

impl AppState {
    fn theme(&self) -> Theme {
        Theme::CatppuccinFrappe
    }

    fn update(&mut self, message: Message) {
        match self {
            AppState::MainMenu => match message {
                Message::SinglePlayer => {
                    *self = AppState::SinglePlayer(Game::default());
                }
                Message::Multiplayer => {}
                _ => {}
            },
            AppState::SinglePlayer(game) => {
                match message {
                    Message::Reveal(row, col) => {
                        if !game.running && !game.game_over && !game.game_won {
                            // only generate bombs after first click
                            game.generate_bombs(row, col);
                            game.compute_cell_numbers();
                            game.running = true;
                        }
                        game.flood_fill(row, col);
                        game.check_game_won();
                    }
                    Message::Flag(row, col) => {
                        if game.game_over || game.game_won {
                            return;
                        }
                        if !game.grid[row][col].is_flaged && !game.grid[row][col].is_revealed {
                            // don't allow more flags than mines
                            if game.flags == game.mine_count {
                                return;
                            }
                            game.grid[row][col].is_flaged = true;
                            game.flags += 1;
                        } else if game.grid[row][col].is_flaged {
                            game.grid[row][col].is_flaged = false;
                            game.flags -= 1;
                        }
                    }
                    Message::NewGame => {
                        *game = Game::default();
                    }
                    Message::Tick => {
                        game.seconds += 1;
                    }
                    Message::MainMenu => {
                        *self = AppState::MainMenu;
                    }
                    _ => {}
                }
            }
            AppState::Multiplayer(state) => {}
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self {
            AppState::MainMenu => center(
                column![
                    text("Minesweeper Deluxe").size(50),
                    button("Single Player").on_press(Message::SinglePlayer),
                    button("Multiplayer").on_press(Message::Multiplayer)
                ]
                .spacing(50)
                .width(Length::Fill)
                .align_x(Center),
            )
            .into(),
            AppState::SinglePlayer(game) => {
                let grid_size = game.grid_size;
                let grid = column((0..grid_size).map(|y| {
                    row((0..grid_size).map(|x| {
                        let cell = &game.grid[x][y];
                        let mut number = "".to_string();
                        let mut cell_color = Color::from_rgb(0.5, 0.5, 0.5);
                        let text_color = match cell.number {
                            1 => BLUE,
                            2 => GREEN,
                            3 => RED,
                            4 => DARK_BLUE,
                            5 => DARK_RED,
                            6 => TEAL,
                            7 => BLACK,
                            8 => GRAY,
                            _ => Color::TRANSPARENT,
                        };
                        if cell.is_revealed {
                            cell_color = Color::from_rgb(0.8, 0.8, 0.8);
                            if !cell.is_mine && cell.number > 0 {
                                number = cell.number.to_string();
                            } else if cell.is_mine {
                                number = "💥".to_string();
                            }
                        } else if cell.is_flaged {
                            number = "🚩".to_string();
                        }
                        mouse_area(
                            button(text(number).color(text_color).center())
                                .style(move |_theme, _status| button::Style {
                                    background: Some(iced::Background::Color(cell_color)),
                                    border: iced::Border {
                                        radius: 2.0.into(),
                                        width: 1.0,
                                        color: Color::BLACK,
                                    },
                                    ..button::Style::default()
                                })
                                .width(32)
                                .height(32),
                        )
                        .on_press(Message::Reveal(x, y))
                        .on_right_press(Message::Flag(x, y))
                        .into()
                    }))
                    .into()
                }));

                let mut title_content = "Minesweeper";
                if game.game_over {
                    title_content = "Game Over!";
                } else if game.game_won {
                    title_content = "Game Won!";
                }
                let timer = text(format!("{}:{:02}", game.seconds / 60, game.seconds % 60));
                let title = text(title_content);
                let stats = row![text(format!(
                    "Bombs Remaining: {}",
                    game.mine_count - game.flags
                ))];
                let controls = row![
                    button(text("New Game").size(12).center())
                        .on_press(Message::NewGame)
                        .width(100)
                        .height(20),
                    timer,
                    button(text("Main Menu").size(12).center())
                        .on_press(Message::MainMenu)
                        .width(100)
                        .height(20)
                ]
                .spacing(100);

                center(
                    column![title, controls, grid, stats]
                        .spacing(20)
                        .width(Length::Fill)
                        .align_x(Center),
                )
                .into()
            }
            AppState::Multiplayer(state) => {
                center(column![button("Single Player"), button("Multiplayer")]).into()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        match self {
            AppState::MainMenu => Subscription::none(),
            AppState::SinglePlayer(game) => {
                if game.running && !game.game_over && !game.game_won {
                    time::every(seconds(1)).map(|_| Message::Tick)
                } else {
                    Subscription::none()
                }
            }
            AppState::Multiplayer(state) => Subscription::none(),
        }
    }
}

impl Game {
    fn check_game_won(&mut self) {
        let num_clear_squares = (DEFAULT_GRID_SIZE * DEFAULT_GRID_SIZE) - DEFAULT_MINE_COUNT;
        if self.squares_cleared == num_clear_squares {
            self.game_won = true;
        }
    }

    fn generate_bombs(&mut self, selected_row: usize, selected_col: usize) {
        let mut rng = rand::rng();
        let mut mines_placed = 0;
        while mines_placed < self.mine_count {
            let row = rng.random_range(0..self.grid_size);
            let col = rng.random_range(0..self.grid_size);
            if !self.grid[row][col].is_mine {
                if row == selected_row && col == selected_col {
                    continue;
                }
                self.grid[row][col].is_mine = true;
                mines_placed += 1;
            }
        }
    }

    fn flood_fill(&mut self, row: usize, col: usize) {
        if self.game_over
            || self.game_won
            || self.grid[row][col].is_revealed
            || self.grid[row][col].is_flaged
        {
            return;
        }

        // if it's a mine, game over
        if self.grid[row][col].is_mine {
            self.grid[row][col].is_revealed = true;
            self.game_over = true;
            return;
        }

        let mut queue = VecDeque::new();
        queue.push_back((row, col));

        while let Some((row, col)) = queue.pop_front() {
            if self.grid[row][col].is_revealed {
                continue;
            }

            // reveal square
            self.grid[row][col].is_revealed = true;
            self.squares_cleared += 1;

            // if cell is a 0, push all neighbors to queue
            if self.grid[row][col].number == 0 {
                for (row, col) in self.neighbors(row, col) {
                    if !self.grid[row][col].is_revealed && !self.grid[row][col].is_mine {
                        queue.push_back((row, col));
                    }
                }
            }
        }
    }

    fn compute_cell_numbers(&mut self) {
        for row in 0..self.grid_size {
            for col in 0..self.grid_size {
                // skip if is mine
                if self.grid[row][col].is_mine {
                    continue;
                }
                // get neighbors & count mines
                let mine_count = self
                    .neighbors(row, col)
                    .iter()
                    .filter(|(row, col)| self.grid[*row][*col].is_mine)
                    .count();

                self.grid[row][col].number = mine_count as u8;
            }
        }
    }

    fn neighbors(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::with_capacity(8);

        for row_diff in -1i32..=1 {
            for col_diff in -1i32..=1 {
                if row_diff == 0 && col_diff == 0 {
                    continue;
                }

                let new_row = row as i32 + row_diff;
                let new_col = col as i32 + col_diff;
                if new_row >= 0
                    && new_row < self.grid_size as i32
                    && new_col >= 0
                    && new_col < self.grid_size as i32
                {
                    result.push((new_row as usize, new_col as usize));
                }
            }
        }

        result
    }
}
