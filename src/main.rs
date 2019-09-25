use async_std::{
    io,
    net::{TcpListener, TcpStream},
    prelude::*,
    task,
    fs,
    sync::RwLock,
};
use std::{
    str,
    net::Shutdown,
    time::SystemTime,
    sync::Arc,
};

const MAXCONN: usize = 3;


#[derive(Hash, Eq, PartialEq, Debug, Clone)]
struct Client {
    name: String,
    timein: SystemTime,
    timeout: Option<SystemTime>,
}

impl Client {
    fn new(client_num: usize) -> Client{
        let name = format!("Client {}", client_num);
        Client{name : name, timein: SystemTime::now(), timeout: None}
    }
    fn disconnect(&mut self) {
        self.timeout = Some(SystemTime::now())
    }
}

async fn process(mut stream: TcpStream, client: &Client) -> io::Result<()> {
    println!("Accepted from: {}", stream.peer_addr()?);
    stream.write_all(b"hello what do you want to do\n").await?;
    loop {
        let mut buf = vec![0u8; 1024];
        let (reader, writer) = &mut (&stream, &stream);
        reader.read(&mut buf).await?;
        buf.retain(|&i| i != 0);
        let mut response = str::from_utf8(&buf).unwrap();
        response = response.trim();
        dbg!(buf.len());
        dbg!(response.as_bytes());
        if response == "exit" {
            println!("connection shutdown on stream {}", stream.peer_addr()?);
            stream.shutdown(Shutdown::Both)?;
            break;
        }
        else if response == "ls" {
            let mut send = String::new();
            let mut entries = fs::read_dir("./files").await?;
            while let Some(res) = entries.next().await {
                let entry = res?;
                match entry.file_name().as_os_str().to_str() {
                    None => (),
                    Some(e) => send.push_str(e),
                }
                send.push_str("\n");
            }
            writer.write_all(&send.as_bytes()).await?;
        }

        if buf.len() == 0 {
           println!("Socket closed killing task");
            break;
        }
    }
    Ok(())
}
fn main() -> io::Result<()> {
    task::block_on(async {
        let mut connected: Arc<RwLock<usize>> = Arc::new(RwLock::new(0));
        let mut counter: Arc<RwLock<usize>> = Arc::new(RwLock::new(1));
        println!("{}",*counter.read().await);
        let mut client_list: Arc<Vec<Client>> = Arc::new(Vec::new());
        let listener = TcpListener::bind("127.0.0.1:8080").await?;
        println!("Listening on {}", listener.local_addr()?);

        let mut incoming = listener.incoming();
        let connected_whi = Arc::clone(&connected);
        let counter_whi = Arc::clone(&counter);
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            dbg!(connected.read().await);
            if *connected_whi.read().await < MAXCONN
            {
                *connected_whi.write().await += 1;
                let new_cli = Client::new(*counter_whi.read().await);
                // client_list.push(new_cli.clone());
                *counter_whi.write().await += 1;
                let connected_as = Arc::clone(&connected);
                task::spawn(async move{
                    println!("hello");
                    process(stream, &new_cli).await.unwrap();
                    println!("done with {}", new_cli.name);
                    *connected_as.write().await -= 1;
                });
                
            }
            else 
            {
                println!("not accepting connection connection buffer full")
            }
        }
        Ok(())
    })
}
