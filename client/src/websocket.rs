use iced::futures;
use iced::task::{Never, Sipper, sipper};
use iced::widget::text;

use futures::channel::mpsc;
use futures::sink::SinkExt;
use futures::stream::StreamExt;

use async_tungstenite::tungstenite;
use std::fmt;

pub fn connect() -> impl Sipper<Never, Event> {
    sipper(async |mut output| {
        loop {
            const ECHO_SERVER: &str = "ws://0.0.0.0:8080/ws";

            let (mut websocket, mut input) =
                match async_tungstenite::tokio::connect_async(ECHO_SERVER).await {
                    Ok((websocket, _)) => {
                        dbg!("websocket connected!");
                        let (sender, receiver) = mpsc::channel(100);

                        output.send(Event::Connected(Connection(sender))).await;

                        (websocket.fuse(), receiver)
                    }
                    Err(err) => {
                        dbg!("websocket error: {}", err);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                        continue;
                    }
                };

            loop {
                futures::select! {
                    received = websocket.select_next_some() => {
                        match received {
                            Ok(tungstenite::Message::Text(message)) => {
                                if let Ok(msg) = serde_json::from_str::<shared::WsMsg>(&message) {
                                    output.send(Event::MessageReceived(msg)).await;
                                }
                            }
                            Err(_) => {
                                output.send(Event::Disconnected).await;
                                break;
                            }
                            Ok(_) => {},
                        }
                    }
                    message = input.select_next_some() => {
                        if let Ok(json_msg) = serde_json::to_string(&message) {
                        let result = websocket.send(tungstenite::Message::Text(json_msg.into())).await;

                        if result.is_err() {
                            output.send(Event::Disconnected).await;
                        }

                        }
                    }
                }
            }
        }
    })
}

#[derive(Debug, Clone)]
pub enum Event {
    Connected(Connection),
    Disconnected,
    MessageReceived(shared::WsMsg),
}

#[derive(Debug, Clone)]
pub struct Connection(mpsc::Sender<shared::WsMsg>);

impl Connection {
    pub fn send(&mut self, message: shared::WsMsg) {
        self.0
            .try_send(message)
            .expect("Send message to echo server");
    }
}
