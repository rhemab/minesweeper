#![windows_subsystem = "windows"]

use iced::time::{self, seconds};
use iced::widget::{button, center, column, mouse_area, row, text};
use iced::{Center, Color, Element, Length, Subscription, Task, Theme};

use std::collections::HashSet;

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
    Loading,
    SinglePlayer(shared::MinesweeperGame),
    Multiplayer(MultiplayerState),
}

#[derive(Default)]
struct MultiplayerState {
    game: shared::MinesweeperGame,
    flags: HashSet<(usize, usize)>,
    connection: WebsocketState,
    turn: usize,
    role: usize,
    game_id: String,
    winner: usize,
    player_one_name: String,
    player_two_name: String,
}

#[derive(Default)]
enum WebsocketState {
    Connected(websocket::Connection),
    #[default]
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

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainMenu => {
                *self = AppState::MainMenu;
                return Task::none();
            }
            _ => {}
        }
        match self {
            AppState::MainMenu => match message {
                Message::SinglePlayer => {
                    *self = AppState::SinglePlayer(shared::MinesweeperGame::default());
                    return Task::none();
                }
                Message::Multiplayer => {
                    *self = AppState::Multiplayer(MultiplayerState {
                        game: shared::MinesweeperGame::default(),
                        flags: HashSet::new(),
                        connection: WebsocketState::Disconnected,
                        turn: 1,
                        role: 0,
                        game_id: String::new(),
                        winner: 0,
                        player_one_name: String::new(),
                        player_two_name: String::new(),
                    });
                    return Task::none();
                }
                _ => {
                    return Task::none();
                }
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
                        return Task::none();
                    }
                    Message::Flag(row, col) => {
                        if game.game_over || game.game_won {
                            return Task::none();
                        }
                        if !game.grid[row][col].is_flaged && !game.grid[row][col].is_revealed {
                            // don't allow more flags than mines
                            if game.flags == game.mine_count {
                                return Task::none();
                            }
                            game.grid[row][col].is_flaged = true;
                            game.flags += 1;
                        } else if game.grid[row][col].is_flaged {
                            game.grid[row][col].is_flaged = false;
                            game.flags -= 1;
                        }
                        return Task::none();
                    }
                    Message::NewGame => {
                        *game = shared::MinesweeperGame::default();
                        return Task::none();
                    }
                    Message::Tick => {
                        game.seconds += 1;
                        return Task::none();
                    }
                    _ => {
                        return Task::none();
                    }
                }
            }
            AppState::Loading => {
                match message {
                    Message::Multiplayer => {
                        *self = AppState::Multiplayer(MultiplayerState {
                            game: shared::MinesweeperGame::default(),
                            flags: HashSet::new(),
                            connection: WebsocketState::Disconnected,
                            turn: 1,
                            role: 0,
                            game_id: String::new(),
                            winner: 0,
                            player_one_name: String::new(),
                            player_two_name: String::new(),
                        });
                        return Task::none();
                    }
                    _ => {}
                }
                return Task::none();
            }
            AppState::Multiplayer(state) => match message {
                Message::NewGame => {
                    *self = AppState::Loading;
                    return Task::perform(async {}, |_| Message::Multiplayer);
                }
                Message::WebsocketEvent(event) => match event {
                    websocket::Event::Connected(connection) => {
                        state.connection = WebsocketState::Connected(connection);
                        return Task::none();
                    }
                    websocket::Event::Disconnected => {
                        state.connection = WebsocketState::Disconnected;
                        return Task::none();
                    }
                    websocket::Event::MessageReceived(message) => match message {
                        shared::WsMsg::NewConnection { game_id, role } => {
                            state.game_id = game_id;
                            state.role = role;
                            return Task::none();
                        }
                        shared::WsMsg::GameState {
                            game,
                            player_one_name: _,
                            player_two_name: _,
                            turn,
                        } => {
                            state.game = game;
                            state.turn = turn;
                            // if cell is revealed, remove flag
                            state
                                .flags
                                .retain(|(row, col)| !state.game.grid[*row][*col].is_revealed);
                            return Task::none();
                        }
                        shared::WsMsg::GameOver { winner } => {
                            state.winner = winner;
                            match &mut state.connection {
                                WebsocketState::Connected(conn) => {
                                    conn.send(shared::WsMsg::Close);
                                }
                                _ => {}
                            }
                            return Task::none();
                        }
                        _ => {
                            return Task::none();
                        }
                    },
                },
                Message::Reveal(row, col) => {
                    // return if flagged
                    if state.flags.contains(&(row, col)) {
                        return Task::none();
                    }
                    // return if not your turn
                    if state.turn != state.role {
                        return Task::none();
                    }
                    // return if already revealed
                    if state.game.grid[row][col].is_revealed {
                        return Task::none();
                    }
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
                    return Task::none();
                }
                Message::Flag(row, col) => {
                    if state.game.game_over || state.game.game_won {
                        return Task::none();
                    }
                    if state.game.grid[row][col].is_revealed {
                        return Task::none();
                    }
                    // if already flagged, remove flag
                    if state.flags.contains(&(row, col)) {
                        state.flags.remove(&(row, col));
                    } else {
                        // only add a flag if not revealed
                        // don't allow more flags than mines
                        if state.flags.len() == state.game.mine_count {
                            return Task::none();
                        }
                        state.flags.insert((row, col));
                    }
                    return Task::none();
                }
                _ => {
                    return Task::none();
                }
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
            AppState::Loading => center(text("Connecting...")).into(),
            AppState::Multiplayer(state) => {
                let mut grid = column((0..state.game.height).map(|y| {
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
                        } else if state.flags.contains(&(y, x)) {
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

                let mut player_name = text("");
                if state.role == 1 {
                    if state.player_one_name.is_empty() {
                        player_name = text("Player 1").size(12);
                    } else {
                        player_name = text(&state.player_one_name).size(12);
                    }
                } else if state.role == 2 {
                    if state.player_two_name.is_empty() {
                        player_name = text("Player 2").size(12);
                    } else {
                        player_name = text(&state.player_two_name).size(12);
                    }
                }
                grid = grid.push(player_name);

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
                    state.game.mine_count - state.flags.len()
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
            _ => Subscription::none(),
        }
    }
}
