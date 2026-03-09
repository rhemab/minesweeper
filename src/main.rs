use iced::widget::{button, center, column, row, text};
use iced::{Color, Element};
use rand::prelude::*;

const DEFAULT_GRID_SIZE: usize = 10;

pub fn main() -> iced::Result {
    iced::run(Minesweeper::update, Minesweeper::view)
}

struct Minesweeper {
    grid_size: usize,
    grid: Vec<Vec<Cell>>,
}

impl Default for Minesweeper {
    fn default() -> Self {
        Self {
            grid_size: DEFAULT_GRID_SIZE,
            grid: generate_grid(DEFAULT_GRID_SIZE, DEFAULT_GRID_SIZE, DEFAULT_GRID_SIZE),
        }
    }
}

#[derive(Default, Clone, Debug)]
struct Cell {
    is_revealed: bool,
    is_mine: bool,
    number: u32,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Reveal(usize, usize),
}

impl Minesweeper {
    fn update(&mut self, message: Message) {
        match message {
            Message::Reveal(x, y) => {
                self.grid[x][y].is_revealed = true;
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
        .spacing(1);

        center(grid).into()
    }
}

fn generate_grid(width: usize, height: usize, mine_count: usize) -> Vec<Vec<Cell>> {
    let mut rng = rand::rng();
    let mut grid = vec![vec![Cell::default(); width]; height];

    let mut mines_placed = 0;
    while mines_placed < mine_count {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);
        if !grid[y][x].is_mine {
            grid[y][x].is_mine = true;
            mines_placed += 1;
        }
    }

    grid
}
