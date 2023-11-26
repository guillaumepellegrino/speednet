/// Control messages sent between server and client
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use serde::Serialize;
use crate::args::ArgsClient;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::net::UdpSocket;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
/// List of Messages used between speednet server and client
pub enum Message {
    /// Client starts by greeting the Server with
    /// an Hello message containing the client configuration
    /// on the TCP control connection
    ClientHello(ArgsClient),

    /// Server replies back by greeting the client with
    /// an Hello message containing the Test ID
    /// on the TCP control connection.
    ServerHello(u32),

    /// Client initialize a new data stream with the server (TCP or UDP data stream).
    /// The first argument is the Test ID provided in ServerHello message.
    /// The second argument is the Stream ID.
    ///
    /// For UDP, if the client does not get a ServerInitStream reply, it may
    /// try to resend it again in order to handle packet loss.
    ClientStreamHello(u32, u32),

    /// Server acknowledge than stream is correctly initialized
    /// on TCP or UDP data stream.
    ServerStreamHello,

    /// Client ask Server to start the test when all streams are initialized
    /// on the TCP control connection.
    ClientStartTest,

    /// Server send a test update to the client every second
    /// on the TCP control connection.
    ServerTestUpdate,
}

pub trait MessageIO {
    fn sendmsg(&mut self, msg: &Message) -> Result<()>;
    fn recvmsg(&mut self) -> Result<Message>;
}

impl MessageIO for TcpStream {
    // Send a speednet control message on a TCP Stream
    //
    // The message is stringifyied in JSON and terminated by a NULL character
    // before being sent on the TCP socket.
    fn sendmsg(&mut self, msg: &Message) -> Result<()> {
        let string = serde_json::to_string(msg)
            .wrap_err("Failed to stringify message")?;
        let mut buff = string.into_bytes();
        buff.push(0);

        self.write(&buff)
            .wrap_err("Failed to send message")?;
        self.flush()
            .wrap_err("Failed to flush message")?;

        Ok(())
    }

    // Recv a speednet control message from a TCP Stream
    //
    // The message is received in JSON formated and is delimited by a NULL character.
    fn recvmsg(&mut self) -> Result<Message> {
        let mut buff = vec!(0; 4096);

        // Find the message size
        let readlen = self.peek(&mut buff)
            .wrap_err("Failed to peek message")?;
        if readlen == 0 {
            return Err(eyre!("Connection closed by server"));
        }
        let eof = buff.iter().position(|x| *x == 0)
            .ok_or(eyre!("Recv message has no end"))?;

        // Read the exact message size
        buff.truncate(eof + 1);
        self.read_exact(&mut buff)
            .wrap_err("Failed to read message")?;
        buff.pop();

        // Parse the message
        let string = std::str::from_utf8(&buff)
            .wrap_err("Received message is not UTF-8")?;
        let msg = serde_json::from_str(string)
            .wrap_err("Failed to parse message")?;

        Ok(msg)
    }
}

impl MessageIO for UdpSocket {
    // Send a speednet control message on a TCP Stream
    //
    // The message is stringifyied in JSON and terminated by a NULL character
    // before being sent on the TCP socket.
    fn sendmsg(&mut self, msg: &Message) -> Result<()> {
        let string = serde_json::to_string(msg)
            .wrap_err("Failed to stringify message")?;
        let mut buff = string.into_bytes();
        buff.push(0);

        self.send(&buff)
            .wrap_err("Failed to send message")?;

        Ok(())
    }

    // Recv a speednet control message from a TCP Stream
    //
    // The message is received in JSON formated and is delimited by a NULL character.
    fn recvmsg(&mut self) -> Result<Message> {
        let mut buff = vec!(0; 4096);

        // Find the message size
        let readlen = self.peek(&mut buff)
            .wrap_err("Failed to peek message")?;
        if readlen == 0 {
            return Err(eyre!("Connection closed by server"));
        }
        let eof = buff.iter().position(|x| *x == 0)
            .ok_or(eyre!("Recv message has no end"))?;

        // Read the exact message size
        buff.truncate(eof + 1);
        self.recv(&mut buff)
            .wrap_err("Failed to read message")?;
        buff.pop();

        // Parse the message
        let string = std::str::from_utf8(&buff)
            .wrap_err("Received message is not UTF-8")?;
        let msg = serde_json::from_str(string)
            .wrap_err("Failed to parse message")?;

        Ok(msg)
    }
}
