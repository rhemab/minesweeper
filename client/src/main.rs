#![windows_subsystem = "windows"]

use iced::time::{self, seconds};
use iced::widget::{button, center, column, mouse_area, row, text};
use iced::{Center, Color, Element, Length, Subscription, Theme};

use shared;

mod websocket;

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
    SinglePlayer(shared::MinesweeperGame),
    Multiplayer(MultiplayerState),
}

struct MultiplayerState {
    game: shared::MinesweeperGame,
    connection: WebsocketState,
    turn: usize,
    role: usize,
    game_id: String,
    winner: usize,
}

enum WebsocketState {
    Connected(websocket::Connection),
    Disconnected,
}

#[derive(Debug, Clone)]
enum Message {
    MainMenu,
    SinglePlayer,
    Multiplayer,
    Reveal(usize, usize),
    Flag(usize, usize),
    NewGame,
    Tick,
    WebsocketEvent(websocket::Event),
}

impl AppState {
    fn theme(&self) -> Theme {
        Theme::CatppuccinFrappe
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::MainMenu => {
                *self = AppState::MainMenu;
                return;
            }
            _ => {}
        }
        match self {
            AppState::MainMenu => match message {
                Message::SinglePlayer => {
                    *self = AppState::SinglePlayer(shared::MinesweeperGame::default());
                }
                Message::Multiplayer => {
                    *self = AppState::Multiplayer(MultiplayerState {
                        game: shared::MinesweeperGame::default(),
                        connection: WebsocketState::Disconnected,
                        turn: 1,
                        role: 0,
                        game_id: String::new(),
                        winner: 0,
                    })
                }
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
                        *game = shared::MinesweeperGame::default();
                    }
                    Message::Tick => {
                        game.seconds += 1;
                    }
                    _ => {}
                }
            }
            AppState::Multiplayer(state) => match message {
                Message::NewGame => {}
                Message::WebsocketEvent(event) => match event {
                    websocket::Event::Connected(connection) => {
                        state.connection = WebsocketState::Connected(connection);
                    }
                    websocket::Event::Disconnected => {
                        state.connection = WebsocketState::Disconnected;
                    }
                    websocket::Event::MessageReceived(message) => match message {
                        shared::WsMsg::NewConnection { game_id, role } => {
                            state.game_id = game_id;
                            state.role = role;
                        }
                        shared::WsMsg::GameState {
                            game,
                            player_one_name: _,
                            player_two_name: _,
                            turn,
                        } => {
                            state.game = game;
                            state.turn = turn;
                        }
                        shared::WsMsg::GameOver { winner } => {
                            state.winner = winner;
                            match &mut state.connection {
                                WebsocketState::Connected(conn) => {
                                    conn.send(shared::WsMsg::Close);
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    },
                },
                Message::Reveal(row, col) => {
                    // send move to server
                    match &mut state.connection {
                        WebsocketState::Connected(conn) => {
                            conn.send(shared::WsMsg::NewMove {
                                row,
                                col,
                                game_id: state.game_id.clone(),
                            });
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
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
                let grid = column((0..game.height).map(|y| {
                    row((0..game.width).map(|x| {
                        let cell = &game.grid[y][x];
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
                        .on_press(Message::Reveal(y, x))
                        .on_right_press(Message::Flag(y, x))
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
                let stats = row![
                    text(format!("Bombs Remaining: {}", game.mine_count - game.flags)).size(12)
                ];
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
                        .spacing(10)
                        .width(Length::Fill)
                        .align_x(Center),
                )
                .into()
            }
            AppState::Multiplayer(state) => {
                let grid = column((0..state.game.height).map(|y| {
                    row((0..state.game.width).map(|x| {
                        let cell = &state.game.grid[y][x];
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
                        .on_press(Message::Reveal(y, x))
                        .on_right_press(Message::Flag(y, x))
                        .into()
                    }))
                    .into()
                }));

                let online_status;
                let online_status_color;
                match state.connection {
                    WebsocketState::Disconnected => {
                        online_status = "Multiplayer Disconnected";
                        online_status_color = RED;
                    }
                    WebsocketState::Connected(_) => {
                        online_status = "Multiplayer Connected";
                        online_status_color = GREEN;
                    }
                };
                let bombs_remaining = text(format!(
                    "Bombs Remaining: {}",
                    state.game.mine_count - state.game.flags
                ))
                .size(12);
                let online_status = row![
                    text(online_status).size(12).color(online_status_color),
                    bombs_remaining
                ]
                .spacing(100);

                let mut title = "Minesweeper";
                if state.game.game_over {
                    title = "Game Over!";
                } else if state.game.game_won {
                    title = "Game Won!";
                }
                if state.winner == state.role {
                    title = "Game Won!";
                }
                let title = text(title);
                let timer = text(format!(
                    "{}:{:02}",
                    state.game.seconds / 60,
                    state.game.seconds % 60
                ));
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
                let your_turn;
                if state.turn == state.role {
                    your_turn = "Your Turn";
                } else {
                    your_turn = "";
                }

                center(
                    column![title, controls, grid, online_status, your_turn]
                        .spacing(10)
                        .width(Length::Fill)
                        .align_x(Center),
                )
                .into()
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
            AppState::Multiplayer(_) => {
                Subscription::run(websocket::connect).map(Message::WebsocketEvent)
            }
        }
    }
}
