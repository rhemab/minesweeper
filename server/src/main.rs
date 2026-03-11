use axum::{
    Router,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::any,
};
use std::net::SocketAddr;
use std::ops::ControlFlow;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use axum::extract::connect_info::ConnectInfo;

use futures_util::{sink::SinkExt, stream::StreamExt};
use shared;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Default)]
struct AppState {
    games: HashMap<String, Game>,
    game_waiting: String,
}

struct Game {
    minesweeper: shared::MinesweeperGame,
    player_one: Player,
    player_two: Player,
    turn: usize,
    tx: broadcast::Sender<shared::WsMsg>,
}

#[derive(Default)]
struct Player {
    name: String,
    connected: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = AppState::default();
    let app_state = Arc::new(Mutex::new(app_state));

    // separate task to keep time
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut app_state = app_state_clone.lock().await;
            for game in app_state.games.values_mut() {
                if game.minesweeper.running {
                    game.minesweeper.seconds += 1;
                    let msg = shared::WsMsg::GameState {
                        game: game.minesweeper.clone(),
                        player_one_name: game.player_one.name.clone(),
                        player_two_name: game.player_two.name.clone(),
                        turn: game.turn,
                    };
                    if let Err(err) = game.tx.send(msg) {
                        error!("Error sending over channel: {}", err);
                    }
                }
            }
        }
    });

    // build our application with some routes
    let app = Router::new()
        .route("/ws", any(ws_handler))
        .with_state(app_state.clone());

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    let _ = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await;
}

async fn ws_handler(
    State(app_state): State<Arc<Mutex<AppState>>>,
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    println!("{addr} connected.");
    ws.on_upgrade(move |socket| handle_socket(socket, addr, app_state))
}

async fn handle_socket(socket: WebSocket, who: SocketAddr, app_state: Arc<Mutex<AppState>>) {
    let (tx, mut rx) = broadcast::channel::<shared::WsMsg>(32);
    let (mut sender, mut receiver) = socket.split();
    let mut game_id = String::new();
    let mut role = 1;

    {
        let mut app_state = app_state.lock().await;
        if !app_state.game_waiting.is_empty() {
            // join game
            let waiting_game_id = app_state.game_waiting.clone();
            if let Some(game) = app_state.games.get_mut(&waiting_game_id) {
                rx = game.tx.subscribe();
                game_id = waiting_game_id.clone();
                game.player_two.connected = true;
                app_state.game_waiting.clear();
                role = 2;
            }
        }
        if game_id.is_empty() {
            // create a new game
            let new_game = Game {
                minesweeper: shared::MinesweeperGame::default(),
                player_one: Player {
                    name: String::new(),
                    connected: true,
                },
                player_two: Player::default(),
                turn: 1,
                tx,
            };
            rx = new_game.tx.subscribe();
            let new_game_id = Uuid::new_v4().to_string();
            game_id = new_game_id.clone();
            app_state.game_waiting = game_id.clone();
            app_state.games.insert(new_game_id, new_game);
        }

        // send new connection msg
        let msg = shared::WsMsg::NewConnection {
            game_id: game_id.clone(),
            role: role,
        };
        if let Ok(json_msg) = serde_json::to_string(&msg) {
            if let Err(err) = sender.send(Message::Text(json_msg.into())).await {
                error!("ws error sending initial msg: {}", err);
            }
        }

        // send initial game state
        if let Some(game) = app_state.games.get(&game_id) {
            let msg = shared::WsMsg::GameState {
                game: game.minesweeper.clone(),
                player_one_name: game.player_one.name.clone(),
                player_two_name: game.player_two.name.clone(),
                turn: game.turn,
            };
            if let Ok(json_msg) = serde_json::to_string(&msg) {
                if let Err(err) = sender.send(Message::Text(json_msg.into())).await {
                    error!("ws error sending initial msg: {}", err);
                }
            }
        }
        info!("Games: {}", app_state.games.len());
    }

    // receive messages on the channel and
    // send messages on the socket
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json_msg) = serde_json::to_string(&msg) {
                // In case of any websocket error, we exit.
                if sender.send(Message::Text(json_msg.into())).await.is_err() {
                    return 1;
                }
            }
        }

        0
    });

    // receive messages on the socket
    let app_state_clone = app_state.clone();
    let mut recv_task = tokio::spawn(async move {
        let mut cnt = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            cnt += 1;
            if process_message(msg, app_state_clone.clone(), role)
                .await
                .is_break()
            {
                break;
            }
        }
        cnt
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(a) => println!("{a} messages sent to {who}"),
                Err(a) => println!("Error sending messages {a:?}")
            }
            recv_task.abort();
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(b) => println!("Received {b} messages"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
            send_task.abort();
        }
    }

    // returning from the handler closes the websocket connection
    println!("Websocket context {who} destroyed");
    let mut app_state = app_state.lock().await;
    if let Some(game) = app_state.games.get_mut(&game_id) {
        if role == 1 {
            game.player_one.connected = false;
            let game_over = shared::WsMsg::GameOver { winner: 2 };
            if let Err(err) = game.tx.send(game_over) {
                error!("Error sending over channel: {}", err);
            }
        } else if role == 2 {
            game.player_two.connected = false;
            let game_over = shared::WsMsg::GameOver { winner: 1 };
            if let Err(err) = game.tx.send(game_over) {
                error!("Error sending over channel: {}", err);
            }
        }
        if !game.player_one.connected && !game.player_two.connected {
            app_state.games.remove_entry(&game_id);
        }
    }
    info!("Games: {}", app_state.games.len());
}

async fn process_message(
    msg: Message,
    app_state: Arc<Mutex<AppState>>,
    role: usize,
) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            if let Ok(ws_msg) = serde_json::from_str::<shared::WsMsg>(&t) {
                match ws_msg {
                    shared::WsMsg::NewMove { row, col, game_id } => {
                        let mut app_state = app_state.lock().await;
                        if let Some(game) = app_state.games.get_mut(&game_id) {
                            if !game.minesweeper.game_won
                                && !game.minesweeper.game_over
                                && game.player_one.connected
                                && game.player_two.connected
                            {
                                // make move
                                if game.turn == 1 && role == 1 {
                                    // only generate bombs after first click
                                    if !game.minesweeper.running {
                                        game.minesweeper.generate_bombs(row, col);
                                        game.minesweeper.compute_cell_numbers();
                                        game.minesweeper.running = true;
                                    }
                                    game.minesweeper.flood_fill(row, col);
                                    // send game state back to clients
                                    let new_game_state = shared::WsMsg::GameState {
                                        game: game.minesweeper.clone(),
                                        player_one_name: game.player_one.name.clone(),
                                        player_two_name: game.player_two.name.clone(),
                                        turn: 0,
                                    };
                                    if let Err(err) = game.tx.send(new_game_state) {
                                        error!("Error sending over channel: {}", err);
                                    }
                                    if game.minesweeper.game_over {
                                        let game_over = shared::WsMsg::GameOver { winner: 2 };
                                        if let Err(err) = game.tx.send(game_over) {
                                            error!("Error sending over channel: {}", err);
                                        }
                                        return ControlFlow::Break(());
                                    }
                                    game.minesweeper.check_game_won();
                                    if game.minesweeper.game_won {
                                        let game_over = shared::WsMsg::GameOver { winner: 1 };
                                        if let Err(err) = game.tx.send(game_over) {
                                            error!("Error sending over channel: {}", err);
                                        }
                                        return ControlFlow::Break(());
                                    }
                                    game.turn += 1;
                                    // send game state back to clients
                                    let new_game_state = shared::WsMsg::GameState {
                                        game: game.minesweeper.clone(),
                                        player_one_name: game.player_one.name.clone(),
                                        player_two_name: game.player_two.name.clone(),
                                        turn: game.turn,
                                    };
                                    if let Err(err) = game.tx.send(new_game_state) {
                                        error!("Error sending over channel: {}", err);
                                    }
                                } else if game.turn == 2 && role == 2 {
                                    game.minesweeper.flood_fill(row, col);
                                    // send game state back to clients
                                    let new_game_state = shared::WsMsg::GameState {
                                        game: game.minesweeper.clone(),
                                        player_one_name: game.player_one.name.clone(),
                                        player_two_name: game.player_two.name.clone(),
                                        turn: 0,
                                    };
                                    if let Err(err) = game.tx.send(new_game_state) {
                                        error!("Error sending over channel: {}", err);
                                    }
                                    if game.minesweeper.game_over {
                                        let game_over = shared::WsMsg::GameOver { winner: 1 };
                                        if let Err(err) = game.tx.send(game_over) {
                                            error!("Error sending over channel: {}", err);
                                        }
                                        return ControlFlow::Break(());
                                    }
                                    game.minesweeper.check_game_won();
                                    if game.minesweeper.game_won {
                                        let game_over = shared::WsMsg::GameOver { winner: 2 };
                                        if let Err(err) = game.tx.send(game_over) {
                                            error!("Error sending over channel: {}", err);
                                        }
                                        return ControlFlow::Break(());
                                    }
                                    game.turn -= 1;
                                    // send game state back to clients
                                    let new_game_state = shared::WsMsg::GameState {
                                        game: game.minesweeper.clone(),
                                        player_one_name: game.player_one.name.clone(),
                                        player_two_name: game.player_two.name.clone(),
                                        turn: game.turn,
                                    };
                                    if let Err(err) = game.tx.send(new_game_state) {
                                        error!("Error sending over channel: {}", err);
                                    }
                                }
                            }
                            return ControlFlow::Continue(());
                        }
                    }
                    _ => {}
                }
            }
        }
        Message::Close(_) => {
            return ControlFlow::Break(());
        }

        _ => {}
    }
    ControlFlow::Continue(())
}
