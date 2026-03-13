use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WsMsg {
    // receive from client
    NewMove {
        row: usize,
        col: usize,
        game_id: String,
    },
    Close,

    // receive from server
    GameOver {
        winner: usize,
    },
    NewConnection {
        game_id: String,
        role: usize,
    },
    GameState {
        game: MinesweeperGame,
        player_one: Player,
        player_two: Player,
        turn: usize,
    },
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    pub name: String,
    pub connected: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MinesweeperGame {
    pub height: usize,
    pub width: usize,
    pub grid: Vec<Vec<Cell>>,
    pub squares_cleared: usize,
    pub mine_count: usize,
    pub flags: usize,
    pub game_over: bool,
    pub game_won: bool,
    pub running: bool,
    pub seconds: u32,
}

impl Default for MinesweeperGame {
    fn default() -> Self {
        let height = 10;
        let width = 30;
        let game = Self {
            height,
            width,
            grid: vec![vec![Cell::default(); width]; height],
            squares_cleared: 0,
            mine_count: height * 3,
            flags: 0,
            game_over: false,
            game_won: false,
            running: false,
            seconds: 0,
        };
        game
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Cell {
    pub is_revealed: bool,
    pub is_mine: bool,
    pub is_flaged: bool,
    pub number: u8,
}

impl MinesweeperGame {
    pub fn new(height: usize, width: usize) -> Self {
        let game = Self {
            height,
            width,
            grid: vec![vec![Cell::default(); width]; height],
            squares_cleared: 0,
            mine_count: width * 2,
            flags: 0,
            game_over: false,
            game_won: false,
            running: false,
            seconds: 0,
        };
        game
    }

    pub fn check_game_won(&mut self) {
        let num_clear_squares = (self.height * self.width) - self.mine_count;
        if self.squares_cleared == num_clear_squares {
            self.game_won = true;
        }
    }

    pub fn generate_bombs(&mut self, selected_row: usize, selected_col: usize) {
        let mut rng = rand::rng();
        let mut mines_placed = 0;
        while mines_placed < self.mine_count {
            let row = rng.random_range(0..self.height);
            let col = rng.random_range(0..self.width);
            if !self.grid[row][col].is_mine {
                if row == selected_row && col == selected_col {
                    continue;
                }
                self.grid[row][col].is_mine = true;
                mines_placed += 1;
            }
        }
    }

    pub fn flood_fill(&mut self, row: usize, col: usize) {
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

    pub fn compute_cell_numbers(&mut self) {
        for row in 0..self.height {
            for col in 0..self.width {
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

    pub fn neighbors(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::with_capacity(8);

        for row_diff in -1i32..=1 {
            for col_diff in -1i32..=1 {
                if row_diff == 0 && col_diff == 0 {
                    continue;
                }

                let new_row = row as i32 + row_diff;
                let new_col = col as i32 + col_diff;
                if new_row >= 0
                    && new_row < self.height as i32
                    && new_col >= 0
                    && new_col < self.width as i32
                {
                    result.push((new_row as usize, new_col as usize));
                }
            }
        }

        result
    }
}
