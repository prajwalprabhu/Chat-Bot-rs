use serde_derive::{Deserialize, Serialize};
use tokio::{
    io::{stdin, AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    member: String,
    chat: String,
}
fn buff_to_string(buff: [u8; 1024]) -> Option<String> {
    let mut buffer = Vec::<u8>::new();
    if buff[0] == 0 {
        None
    } else {
        for &i in buff.iter() {
            if i != 0 {
                buffer.push(i);
            } else {
                break;
            }
        }
    }
    Some(String::from_utf8_lossy(&buffer).to_string())
}
fn buff_to_message(buff: [u8; 1024]) -> Option<Message> {
    if buff[0] == 0 {
        return None;
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
    Some(result)
}

async fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buff = [0; 1024];
    // let mut std = stdin();

    stream.read(&mut buff).await?;
    let mut result = Message {
        chat: "".to_string(),
        member: "".to_string(),
    };
    match buff_to_message(buff) {
        Some(res) => result = res,
        None => {}
    }
    if result.member == "root".to_string() {
        let message = serde_json::to_string(&Message {
            member: "test".to_string(),
            chat: "Welcome".to_string(),
        })?;
        stream.write(message.as_bytes()).await?;
        stream.flush().await?;
        print!("Flushed");
    }
    let (reader, writer) = stream.into_split();
    let recv_thread = tokio::spawn(recv(reader));
    let send_thread = tokio::spawn(send(writer));
    recv_thread.await??;
    send_thread.await??;
    Ok(())
}
async fn recv(mut reader: OwnedReadHalf) -> std::io::Result<()> {
    loop {
        let mut buff = [0; 1024];
        reader.read(&mut buff).await?;
        match buff_to_message(buff) {
            Some(result) => println!("{} : {}", result.member, result.chat),
            None => continue,
        }
    }
    Ok(())
}

async fn send(mut writer: OwnedWriteHalf) -> std::io::Result<()> {
    loop {
        println!("Enter your message :");
        let mut buf = [0; 1024];
        let mut std = stdin();
        std.read(&mut buf).await?;
        let res = buff_to_string(buf).unwrap_or("".to_string());
        if res.to_lowercase() == "exit" {
            println!("Exit");
            return Ok(());
        } else {
            let message = serde_json::to_string(&Message {
                member: "test".to_string(),
                chat: res,
            })?;
            writer.write_all(message.as_bytes()).await?;
            writer.flush().await?;
        }
    }
    Ok(())
}
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpStream::connect("127.0.0.1:8080").await?;
    handle_client(listener).await?;
    Ok(())
}
