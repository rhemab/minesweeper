use iced::widget::{button, center, column, row, text};
use iced::{Center, Color, Element, Length};
use rand::prelude::*;
use std::collections::VecDeque;

const DEFAULT_GRID_SIZE: usize = 10;

pub fn main() -> iced::Result {
    iced::run(Minesweeper::update, Minesweeper::view)
}

struct Minesweeper {
    grid_size: usize,
    grid: Vec<Vec<Cell>>,
    game_over: bool,
}

impl Default for Minesweeper {
    fn default() -> Self {
        let mut grid = Self {
            grid_size: DEFAULT_GRID_SIZE,
            grid: generate_grid(DEFAULT_GRID_SIZE, DEFAULT_GRID_SIZE),
            game_over: false,
        };

        grid.compute_cell_numbers();

        grid
    }
}

#[derive(Default, Clone, Debug)]
struct Cell {
    is_revealed: bool,
    is_mine: bool,
    number: u8,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Reveal(usize, usize),
}

impl Minesweeper {
    fn update(&mut self, message: Message) {
        match message {
            Message::Reveal(row, col) => {
                self.flood_fill(row, col);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let grid_size = self.grid_size;
        let grid = column((0..grid_size).map(|y| {
            row((0..grid_size).map(|x| {
                let cell = &self.grid[x][y];
                let mut number = "".to_string();
                let mut color = Color::from_rgb(0.5, 0.5, 0.5);
                if cell.is_revealed {
                    color = Color::from_rgb(0.8, 0.8, 0.8);
                    if !cell.is_mine && cell.number > 0 {
                        number = cell.number.to_string();
                    } else if cell.is_mine {
                        number = "*".to_string();
                    }
                };
                button(text(number).center())
                    .style(move |_theme, _status| button::Style {
                        background: Some(iced::Background::Color(color)),
                        border: iced::Border {
                            radius: 2.0.into(),
                            width: 1.0,
                            color: Color::BLACK,
                        },
                        ..button::Style::default()
                    })
                    .width(32)
                    .height(32)
                    .on_press(Message::Reveal(x, y))
                    .into()
            }))
            // .spacing(1)
            .into()
        }))
        .width(Length::Fill)
        .align_x(Center)
        .spacing(1);

        let title = text("Minesweeper!").width(Length::Fill).align_x(Center);

        center(column![title, grid].spacing(20)).into()
    }

    fn flood_fill(&mut self, row: usize, col: usize) {
        if self.grid[row][col].is_revealed {
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

            self.grid[row][col].is_revealed = true;

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

fn generate_grid(grid_size: usize, mine_count: usize) -> Vec<Vec<Cell>> {
    let mut rng = rand::rng();
    let mut grid = vec![vec![Cell::default(); grid_size]; grid_size];

    let mut mines_placed = 0;
    while mines_placed < mine_count {
        let x = rng.random_range(0..grid_size);
        let y = rng.random_range(0..grid_size);
        if !grid[y][x].is_mine {
            grid[y][x].is_mine = true;
            mines_placed += 1;
        }
    }

    grid
}
