use crate::database::Database;
use crate::protocol::{Command, Response};
use bytes::{BufMut, BytesMut};
use log::{debug, error, info};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// TCP server for JSON database
pub struct TcpServer {
    database: Arc<Database>,
    address: String,
}

impl TcpServer {
    /// Create a new TCP server
    pub fn new(database: Arc<Database>, address: String) -> Self {
        Self { database, address }
    }

    /// Start the server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.address).await?;
        info!("Server started on {}", self.address);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New connection from {}", addr);
                    let db = Arc::clone(&self.database);
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, db).await {
                            error!("Error handling connection from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single TCP connection
async fn handle_connection(mut stream: TcpStream, database: Arc<Database>) -> Result<(), String> {
    let mut buffer = BytesMut::with_capacity(4096);

    loop {
        // Read data from socket
        match stream.read_buf(&mut buffer).await {
            Ok(0) => {
                debug!("Connection closed by client");
                break;
            }
            Ok(n) => {
                debug!("Received {} bytes", n);
            }
            Err(e) => return Err(format!("Read error: {}", e)),
        }

        // Process messages in buffer
        while let Some((command, remaining)) = parse_message(&buffer)? {
            buffer = remaining;

            debug!("Received command: {}", command);

            // Execute command
            let response = database.execute_command(command).await;
            debug!("Response: {}", response);

            // Send response
            send_response(&mut stream, response).await?;
        }
    }

    Ok(())
}

/// Simple communication protocol based on length + payload
/// Format: [length:4 bytes][JSON payload]
fn parse_message(buffer: &BytesMut) -> Result<Option<(Command, BytesMut)>, String> {
    if buffer.len() < 4 {
        return Ok(None); // Not enough data for length
    }

    let mut length_bytes = [0u8; 4];
    length_bytes.copy_from_slice(&buffer[0..4]);
    let message_length = u32::from_be_bytes(length_bytes) as usize;

    if buffer.len() < 4 + message_length {
        return Ok(None); // Not enough data for complete message
    }

    // Extract the payload
    let payload = &buffer[4..4 + message_length];

    // Deserialize the command using JSON
    let payload_str =
        std::str::from_utf8(payload).map_err(|e| format!("Non-UTF-8 payload: {}", e))?;
    let command: Command = serde_json::from_str(payload_str)
        .map_err(|e| format!("JSON deserialization error: {}", e))?;

    // Create the remaining buffer
    let mut remaining = BytesMut::new();
    if buffer.len() > 4 + message_length {
        remaining.extend_from_slice(&buffer[4 + message_length..]);
    }

    Ok(Some((command, remaining)))
}

/// Send a response to the client
async fn send_response(stream: &mut TcpStream, response: Response) -> Result<(), String> {
    // Serialize response using JSON
    let payload_str = serde_json::to_string(&response)
        .map_err(|e| format!("JSON serialization error: {}", e))?;
    let payload = payload_str.as_bytes();
    let payload_length = payload.len() as u32;

    // Create message with length + payload
    let mut message = BytesMut::with_capacity(4 + payload.len());
    message.put_u32(payload_length);
    message.extend_from_slice(payload);

    // Send the message
    stream
        .write_all(&message)
        .await
        .map_err(|e| format!("Send error: {}", e))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("Flush error: {}", e))?;

    Ok(())
}

/// TCP client for JSON database
pub struct TcpClient {
    stream: TcpStream,
}

impl TcpClient {
    /// Connect to server
    pub async fn connect(address: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(address)
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;
        info!("Connected to server {}", address);
        Ok(Self { stream })
    }

    /// Send a command and receive the response
    pub async fn send_command(&mut self, command: Command) -> Result<Response, String> {
        debug!("Sending command: {}", command);

        // Serialize command using JSON
        let payload_str = serde_json::to_string(&command)
            .map_err(|e| format!("JSON serialization error: {}", e))?;
        let payload = payload_str.as_bytes();
        let payload_length = payload.len() as u32;

        // Create message with length + payload
        let mut message = BytesMut::with_capacity(4 + payload.len());
        message.put_u32(payload_length);
        message.extend_from_slice(payload);

        // Send the message
        self.stream
            .write_all(&message)
            .await
            .map_err(|e| format!("Send error: {}", e))?;
        self.stream
            .flush()
            .await
            .map_err(|e| format!("Flush error: {}", e))?;

        // Receive the response
        let response = self.receive_response().await?;
        debug!("Response received: {}", response);

        Ok(response)
    }

    /// Receive a response from the server
    async fn receive_response(&mut self) -> Result<Response, String> {
        // Read the length
        let mut length_bytes = [0u8; 4];
        self.stream
            .read_exact(&mut length_bytes)
            .await
            .map_err(|e| format!("Length read error: {}", e))?;
        let message_length = u32::from_be_bytes(length_bytes) as usize;

        // Read the payload
        let mut payload = vec![0u8; message_length];
        self.stream
            .read_exact(&mut payload)
            .await
            .map_err(|e| format!("Payload read error: {}", e))?;

        // Deserialize response using JSON
        let payload_str =
            std::str::from_utf8(&payload).map_err(|e| format!("Non-UTF-8 payload: {}", e))?;
        let response: Response = serde_json::from_str(payload_str)
            .map_err(|e| format!("JSON deserialization error: {}", e))?;
        Ok(response)
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<(), String> {
        self.stream
            .shutdown()
            .await
            .map_err(|e| format!("Close error: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Command;
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_tcp_communication() {
        // Start the server in background
        let database = Arc::new(Database::new());
        let _server = TcpServer::new(database, "127.0.0.1:0".to_string());

        // For this test, use a fixed address
        let database = Arc::new(Database::new());
        let server = TcpServer::new(database, "127.0.0.1:8081".to_string());

        tokio::spawn(async move {
            let _ = server.start().await;
        });

        // Wait for server to start
        sleep(Duration::from_millis(100)).await;

        // Test the client
        let mut client = TcpClient::connect("127.0.0.1:8081").await.unwrap();

        // Test SET
        let set_cmd = Command::Set {
            key: "test".to_string(),
            value: json!({"hello": "world"}),
        };
        let response = client.send_command(set_cmd).await.unwrap();
        assert!(matches!(response, Response::Ok(None)));

        // Test GET
        let get_cmd = Command::Get {
            key: "test".to_string(),
        };
        let response = client.send_command(get_cmd).await.unwrap();
        assert!(matches!(response, Response::Ok(Some(_))));

        // Test PING
        let ping_cmd = Command::Ping;
        let response = client.send_command(ping_cmd).await.unwrap();
        assert!(matches!(response, Response::Pong));

        client.close().await.unwrap();
    }
}
