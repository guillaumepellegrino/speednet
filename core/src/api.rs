/// Control messages sent between server and client
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use serde::Serialize;
use crate::args::ArgsClient;
use crate::result;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::net::UdpSocket;

static SPEEDNET_MAGIC: u32 = 0xFEEDCAFE;
static SPEEDNET_MSG_MAXSIZE: u32 = 400000;


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
    ServerTestUpdate(result::ClientResult),
}

pub trait MessageIO {
    fn sendmsg(&mut self, msg: &Message) -> Result<()>;
    fn recvmsg(&mut self) -> Result<Message>;
    fn sendrecvmsg(&mut self, msg: &Message) -> Result<Message> {
        self.sendmsg(msg)?;
        self.recvmsg()
    }
}

impl Message {
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let string = serde_json::to_string(self)
            .wrap_err("Failed to stringify message")?;

        let magic = SPEEDNET_MAGIC.to_ne_bytes();
        let payload = string.into_bytes();
        let payload_len = (payload.len() as u32).to_ne_bytes();
        let mut buff = Vec::with_capacity(magic.len() + payload_len.len() + payload.len());
        buff.extend(magic);
        buff.extend(payload_len);
        buff.extend(payload);
        Ok(buff)
    }

    pub fn deserialize(buff: &[u8]) -> Result<Self> {
        let readlen = buff.len();

        // Check the message magic
        let magic = u32::from_ne_bytes(
            buff.get(0..4)
                .ok_or(eyre!("recvmsg too short"))?
                .try_into().unwrap());
        if magic != SPEEDNET_MAGIC {
            return Err(eyre!("Received message is not a speednet message ! magic: {}", magic));
        }

        // Check the message size
        let payload_len = u32::from_ne_bytes(
            buff.get(4..8)
                .ok_or(eyre!("recvmsg too short"))?
                .try_into().unwrap());
        if payload_len != readlen as u32 - 8 {
            return Err(eyre!("Received message payload len is {} bytes instead of {}", readlen-8, payload_len));
        }

        // Parse payload
        let payload = buff.get(8..readlen)
            .ok_or(eyre!("recvmsg too short"))?;
        let string = std::str::from_utf8(payload)
            .wrap_err("Received message is not UTF-8")?;
        let msg = serde_json::from_str(string)
            .wrap_err("Failed to parse message")?;

        Ok(msg)

    }
}


impl MessageIO for TcpStream {
    // Send a speednet control message on a TCP Stream
    // 
    // Message format is:
    // - [0..4]      Speednet Magic Number
    // - [4..]       Payload LENGTH
    // - [8..LENGTH] Payload message stringifyied in JSON
    fn sendmsg(&mut self, msg: &Message) -> Result<()> {
        let buff = msg.serialize()
            .wrap_err("Failed to serialize message")?;
        self.write(&buff)
            .wrap_err("Failed to send message")?;
        self.flush()
            .wrap_err("Failed to flush message")?;

        Ok(())
    }

    // Recv a speednet control message from a TCP Stream
    //
    // Message format is the same than in sendmsg().
    fn recvmsg(&mut self) -> Result<Message> {
        // read magic number
        let mut magic = [0; 4];
        self.read_exact(&mut magic)
            .wrap_err("Failed to read message magic number")?;
        let magic = u32::from_ne_bytes(magic);
        if magic != SPEEDNET_MAGIC {
            return Err(eyre!("Received message is not a speednet message !"));
        }

        // read payload len
        let mut payload_len = [0; 4];
        self.read_exact(&mut payload_len)
            .wrap_err("Failed to read message payload len")?;
        let payload_len = u32::from_ne_bytes(payload_len);
        if payload_len > SPEEDNET_MSG_MAXSIZE {
            return Err(eyre!("Received message payload len is too long: {} bytes", payload_len));
        }

        // read payload
        let mut payload = vec!(0; payload_len as usize);
        self.read_exact(&mut payload)
            .wrap_err("Failed to read message payload")?;
        
        // Parse the message
        let string = std::str::from_utf8(&payload)
            .wrap_err("Received message is not UTF-8")?;
        let msg = serde_json::from_str(string)
            .wrap_err("Failed to parse message")?;

        Ok(msg)
    }
}

impl MessageIO for UdpSocket {
    // Send a speednet control message on a TCP Stream
    //
    // The message is stringifyied in JSON before being sent on the TCP socket.
    fn sendmsg(&mut self, msg: &Message) -> Result<()> {
        let buff = msg.serialize()
            .wrap_err("Failed to serialize message")?;
        self.send(&buff)
            .wrap_err("Failed to send message")?;

        Ok(())
    }

    // Recv a speednet control message from a TCP Stream
    //
    // The message is received in JSON formated and is delimited by a NULL character.
    fn recvmsg(&mut self) -> Result<Message> {
        // Read the whole message
        let mut buff = vec!(0; 4096);
        let readlen = self.recv(&mut buff)
            .wrap_err("Failed to read message")?;
        if readlen == 0 {
            return Err(eyre!("Connection closed by server"));
        }

        Message::deserialize(&buff[0..readlen])
    }
}
