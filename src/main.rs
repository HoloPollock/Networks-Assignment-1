use async_std::{
    fs,
    fs::File,
    fs::DirEntry,
    io,
    net::{TcpListener, TcpStream},
    prelude::*,
    sync::RwLock,
    task,
};
use std::{net::Shutdown, str, sync::Arc, time::SystemTime};

mod stringutils;

use crate::stringutils::StringUtils;

// Nunber of connection allowed
const MAXCONN: usize = 3;
// Number sent when there is no file found
const NOFILE: i64 = -1;

//Defines What is stored in each Client
#[derive(Debug, Clone)]
struct Client {
    name: String,
    timein: SystemTime,
    timeout: Option<SystemTime>,
}

impl Client {
    fn new(client_num: usize) -> Client {
        let name = format!("Client {}", client_num);
        Client {
            name,
            timein: SystemTime::now(),
            timeout: None,
        }
    }
    fn disconnect(&mut self) {
        self.timeout = Some(SystemTime::now())
    }
    fn respond(&self) -> String {
        let mut respond = String::from("Hello ");
        respond.push_str(&self.name);
        respond.push_str(" what is your name\n");
        respond
    }
}

//allows for some non built in functions with Strings
impl StringUtils for String {
    fn substring(&self, start: usize, len: usize) -> Self {
        self.chars().skip(start).take(len).collect()
    }
    fn remove_whitespace(&mut self) -> Self {
        self.chars().filter(|c| !c.is_whitespace()).collect()
    }
}

// Main Function called to process a connection
async fn process(mut stream: TcpStream, client: &Client) -> io::Result<()> {
    println!("Accepted from: {}", stream.peer_addr()?);
    stream.write_all(client.respond().as_bytes()).await?;
    let mut first = true;
    loop {
        // Create a buffer to read in connection
        let mut buf = vec![0u8; 1024];
        //split stream in to a reader and a writer for better style and code readabilty
        let (reader, writer) = &mut (&stream, &stream);
        //read in response from client
        reader.read(&mut buf).await?;
        //remove all empty space in buffer
        buf.retain(|&i| i != 0);
        //convert to string slice
        let mut response = str::from_utf8(&buf).unwrap();
        response = response.trim();
        // first reponse has different logic than other reponse this handles the reciving of the same
        if first {
            let mut responding = String::from("Hello ");
            responding.push_str(response);
            responding.push_str(" what would you like to do\n");
            writer.write_all(responding.as_bytes()).await?;
            first = false;
        //Hanles closing connection
        } else if response == "exit" {
            println!("connection shutdown on stream {}", stream.peer_addr()?);
            stream.shutdown(Shutdown::Both)?;
            break;
        // lists all files that can be downloaded
        } else if response == "ls" {
            let mut send = String::new();
            //this line assume the code is run from / and not /src
            let mut entries = fs::read_dir("./files").await?;
            //lol for loops don't exist in async yet so this is just a for over an iterator 
            while let Some(res) = entries.next().await {
                let entry = res?;
                // if file exists get it a a string slice and push it to a string slice
                match entry.file_name().as_os_str().to_str() {
                    None => (),
                    Some(e) => send.push_str(e),
                }
                send.push_str("\n");
            }
            // sent to client
            writer.write_all(send.as_bytes()).await?;
        //just send options server can handle to client
        } else if response == "options" {
            let string = String::from("'ls': list all files\n'exit': close connection\n'download <filename>': download file of that name\n'<any string>': get response back from server\n");
            writer.write_all(string.as_bytes()).await?;
        // manages downloading a file
        } else if response.starts_with("download ") {
            let filename = response
                .to_string()
                .remove_whitespace()
                .substring("download".len(), response.len() - 1);
            let mut entries = fs::read_dir("./files").await?;
            let mut file: Option<DirEntry> = None;
            // Go through all files and see if there is one named the same
            while let Some(res) = entries.next().await {
                let entry = res?;
                if entry.file_name().to_string_lossy() == filename {
                    file = Some(entry);
                }
            }
            //if that file exists send the size then send the file
            if let Some(file) = file {
                // get the size of the file
                let buffer_size = file.metadata().await?.len() as i64;
                let mut file_open = File::open("files/".to_string() + &filename).await?;
                // read file in to buffer
                let mut buf = vec![0; buffer_size as usize];
                let _n = file_open.read(&mut buf).await?;
                //send the file size over TCP
                writer.write_all(&buffer_size.to_be_bytes()).await?;
                writer.write_all(b"\n").await?;
                // wait for client to Acknowledge that it has recived the size before sending the file
                // let mut buftemp = 
                reader.read(vec![0u8; 256]).await?;
                
                writer.write_all(&buf).await?;
                writer.write_all(b"download done\n").await?;
            } else {
                writer.write_all(&NOFILE.to_be_bytes()).await?;
                writer.write_all(b"\n").await?;
                let mut buftemp = vec![0u8; 256];
                reader.read(&mut buftemp).await?;
                writer.write_all(b"No File Found\n").await?;
            }
        } else {
            let mut word = String::new();
            word.push_str(response);
            word.push_str("ack\n");
            writer.write_all(word.as_bytes()).await?;
        }

        if buf.is_empty() {
            println!("Socket closed killing task");
            break;
        }
    }
    Ok(())
}

//I know this is stupid and bad but it works enough for the assignment (I should be using channels to communicate not just alot of Arc to clone things)
fn main() -> io::Result<()> {
    task::block_on(async {
        let connected: Arc<RwLock<usize>> = Arc::new(RwLock::new(0));
        let counter: Arc<RwLock<usize>> = Arc::new(RwLock::new(1));
        println!("{}", *counter.read().await);
        let client_list: Arc<RwLock<Vec<Client>>> = Arc::new(RwLock::new(Vec::new()));
        let listener = TcpListener::bind("127.0.0.1:8080").await?;
        println!("Listening on {}", listener.local_addr()?);

        let mut incoming = listener.incoming();
        let connected_whi = Arc::clone(&connected);
        let counter_whi = Arc::clone(&counter);
        let list_whi = Arc::clone(&client_list);
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            if *connected_whi.read().await < MAXCONN {
                *connected_whi.write().await += 1;
                // dbg!(connected.read().await);
                let new_cli = Client::new(*counter_whi.read().await);
                list_whi.write().await.push(new_cli.clone());
                *counter_whi.write().await += 1;
                let connected_as = Arc::clone(&connected);
                // dbg!(&client_list);
                let list_as = Arc::clone(&client_list);
                let counter_as = Arc::clone(&counter);
                task::spawn(async move {
                    let loc = *counter_as.read().await;
                    println!("hello");
                    process(stream, &new_cli).await.unwrap();
                    println!("done with {}", new_cli.name);
                    *connected_as.write().await -= 1;
                    list_as.write().await[loc - 2].disconnect();
                    // dbg!(&list_as);
                });
            } else {
                println!("not accepting connection connection buffer full")
            }
        }
        Ok(())
    })
}
