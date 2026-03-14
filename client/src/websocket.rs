use iced::futures;
use iced::task::{Sipper, sipper};

use futures::channel::mpsc;
use futures::sink::SinkExt;
use futures::stream::StreamExt;

use async_tungstenite::tungstenite;

pub fn connect(url: String) -> impl Sipper<(), Event> {
    sipper(async |mut output| {
        let (mut websocket, mut input) = match async_tungstenite::tokio::connect_async(url).await {
            Ok((websocket, _)) => {
                let (sender, receiver) = mpsc::channel(100);

                output.send(Event::Connected(Connection(sender))).await;

                (websocket.fuse(), receiver)
            }
            Err(err) => {
                dbg!("websocket error: {}", err);
                return;
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
                    match message {
                        shared::WsMsg::Close => {
                            if websocket.close().await.is_err() {
                                output.send(Event::Disconnected).await;
                            }
                            continue;
                        }
                        _ => {}
                    }
                    if let Ok(json_msg) = serde_json::to_string(&message) {
                    let result = websocket.send(tungstenite::Message::Text(json_msg.into())).await;

                    if result.is_err() {
                        output.send(Event::Disconnected).await;
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
        let _ = self.0.try_send(message);
    }
}
