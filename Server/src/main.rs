use serde_derive::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Message {
    member: String,
    chat: String,
}

struct ChatBot {
    stream: Arc<Mutex<Vec<OwnedWriteHalf>>>,
    members: Arc<Mutex<Vec<String>>>,
}

impl ChatBot {
    fn new() -> Self {
        let stream: Arc<Mutex<Vec<OwnedWriteHalf>>> = Arc::new(Mutex::new(Vec::new()));
        let members: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        ChatBot { stream, members }
    }
    async fn start(&mut self) -> std::io::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:8080").await?;
        println!("Started Server on 127.0.0.1:8080");
        let (channel_sender, channel_recv) = channel::<Message>(100);
        let member = Arc::clone(&self.members);
        let stream = Arc::clone(&self.stream);
        let start_thread = tokio::spawn(recv(channel_recv, member, stream.clone()));
        while let Ok((_stream, _)) = listener.accept().await {
            self.add(_stream, channel_sender.clone()).await?;
        }
        start_thread.await.expect("Start thread failed")?;

        Ok(())
    }
    async fn add(
        &mut self,
        mut stream: TcpStream,
        channel: Sender<Message>,
    ) -> std::io::Result<()> {
        let message = serde_json::to_string(&Message {
            member: "root".to_string(),
            chat: "Welcome".to_string(),
        })?;
        let m = message.trim().as_bytes();
        stream.write(m).await?;
        stream.flush().await?;
        let mut buff = [0; 1024];
        stream.read(&mut buff).await?;
        let mut buffer = Vec::<u8>::new();
        for &i in buff.iter() {
            if i != 0 {
                buffer.push(i);
            } else {
                break;
            }
        }
        let result: Message = serde_json::from_str(&String::from_utf8_lossy(&buffer)).unwrap();
        println!("{} Joined to chat", result.member);
        let mut stream_vec = self.stream.lock().await;
        let (reader, writer) = stream.into_split();
        stream_vec.push(writer);
        drop(stream_vec);
        let mut member_vec = self.members.lock().await;
        member_vec.push(result.member);
        drop(member_vec);
        let channel = channel.clone();
        tokio::spawn(listen(reader, channel));
        Ok(())
    }
}

async fn recv(
    mut channel: Receiver<Message>,
    members: Arc<Mutex<Vec<String>>>,
    stream: Arc<Mutex<Vec<OwnedWriteHalf>>>,
) -> std::io::Result<()> {
    while let Some(result) = channel.recv().await {
        let mut members = members.lock().await;
        let mut stream = stream.lock().await;
        for i in 0..members.len() {
            let message = serde_json::to_string(&result).unwrap();
            if stream[i].write(message.as_bytes()).await.is_err() {
                stream.remove(i);
                members.remove(i);
            } else {
                stream[i].flush().await?;
            }
        }
        drop(members);
        drop(stream);
    }
    Ok(())
}

async fn listen(mut stream: OwnedReadHalf, channel: Sender<Message>) -> std::io::Result<()> {
    loop {
        let mut buff = [0; 1024];
        stream.read(&mut buff).await?;
        if buff[0] == 0 {
            continue;
        }
        let mut buffer = Vec::<u8>::new();
        for &i in buff.iter() {
            if i != 0 {
                buffer.push(i);
            } else {
                break;
            }
        }
        let result: Message = serde_json::from_str(&String::from_utf8_lossy(&buffer)).unwrap();
        channel.send(result.clone()).await.unwrap();
        if result.chat.to_uppercase() == "exit" {
            break;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut app = ChatBot::new();
    app.start().await?;
    Ok(())
}
