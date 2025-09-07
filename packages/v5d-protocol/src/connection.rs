//! Connection to a remote v5d process.

use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use interprocess::local_socket::{
    GenericNamespaced, Name, ToNsName,
    tokio::{RecvHalf, SendHalf, Stream, prelude::*},
};
use snafu::Snafu;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    select,
    time::sleep,
};
use tracing::{error, trace, warn};
use vex_v5_serial::{
    connection::{CheckHeader, Connection},
    decode::{Decode, DecodeError},
    packets::{HOST_BOUND_HEADER, cdc2::Cdc2Ack},
    varint::VarU16,
};

#[derive(Debug, Snafu)]
pub enum ConnectionError {
    #[snafu(transparent)]
    Nack {
        source: Cdc2Ack,
    },
    #[snafu(transparent)]
    Io {
        source: io::Error,
    },
    #[snafu(transparent)]
    Encode {
        source: vex_v5_serial::encode::EncodeError,
    },
    #[snafu(transparent)]
    Decode {
        source: vex_v5_serial::decode::DecodeError,
    },
    Timeout,
}

fn get_socket_name() -> Name<'static> {
    "vexide-v5d.sock"
        .to_ns_name::<GenericNamespaced>()
        .expect("socket name should be valid")
}

struct BufStream {
    reader: BufReader<RecvHalf>,
    writer: BufWriter<SendHalf>,
}

impl BufStream {
    fn new(stream: Stream) -> Self {
        let (reader, writer) = stream.split();
        Self {
            reader: BufReader::new(reader),
            writer: BufWriter::new(writer),
        }
    }
}

//TODO: This should just be public in vex-v5-serial
//TODO: There isnt a good reason to force users to reimplement it

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawPacket {
    bytes: Vec<u8>,
    used: bool,
    timestamp: Instant,
}
impl RawPacket {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            used: false,
            timestamp: Instant::now(),
        }
    }

    pub fn is_obsolete(&self, timeout: Duration) -> bool {
        self.timestamp.elapsed() > timeout || self.used
    }

    pub fn check_header<H: CheckHeader>(&self) -> bool {
        H::has_valid_header(self.bytes.clone())
    }

    /// Decodes the packet into the given type.
    /// If successful, marks the packet as used.
    /// # Note
    /// This function will **NOT** fail if the packet has already been used.
    pub fn decode_and_use<D: Decode>(&mut self) -> Result<D, DecodeError> {
        let decoded = D::decode(self.bytes.clone())?;
        self.used = true;
        Ok(decoded)
    }
}
/// Removes old and used packets from the incoming packets buffer.
pub(crate) fn trim_packets(packets: &mut Vec<RawPacket>) {
    trace!("Trimming packets. Length before: {}", packets.len());

    // Remove packets that are obsolete
    packets.retain(|packet| !packet.is_obsolete(Duration::from_secs(2)));

    trace!("Trimmed packets. Length after: {}", packets.len());
}

/// A connection to a remote v5d implementation.
pub struct DaemonConnection {
    stream: BufStream,
    incoming_packets: Vec<RawPacket>,
}

/// Decodes a [`HostBoundPacket`]'s header sequence.
fn decode_header(data: impl IntoIterator<Item = u8>) -> Result<[u8; 2], DecodeError> {
    let mut data = data.into_iter();
    let header = Decode::decode(&mut data)?;
    if header != HOST_BOUND_HEADER {
        return Err(DecodeError::InvalidHeader);
    }
    Ok(header)
}

impl DaemonConnection {
    /// Connect to a running process exposing its device
    pub async fn new() -> Result<Self, ConnectionError> {
        let stream = Stream::connect(get_socket_name()).await?;

        Ok(Self {
            stream: BufStream::new(stream),
            incoming_packets: Vec::new(),
        })
    }

    /// Receives a single packet from the serial port and adds it to the queue of incoming packets.
    async fn receive_one_packet(&mut self) -> Result<(), ConnectionError> {
        // Read the header into an array
        let mut header = [0u8; 2];
        self.stream.reader.read_exact(&mut header).await?;

        // Verify that the header is valid
        if let Err(e) = decode_header(header) {
            warn!(
                "Skipping packet with invalid header: {:x?}. Error: {}",
                header, e
            );
            return Ok(());
        }

        // Create a buffer to store the entire packet
        let mut packet = Vec::from(header);

        // Push the command's ID
        packet.push(self.stream.reader.read_u8().await?);

        // Get the size of the packet
        // We do some extra logic to make sure we only read the necessary amount of bytes
        let first_size_byte = self.stream.reader.read_u8().await?;
        let size = if VarU16::check_wide(first_size_byte) {
            let second_size_byte = self.stream.reader.read_u8().await?;
            packet.extend([first_size_byte, second_size_byte]);

            // Decode the size of the packet
            VarU16::decode(vec![first_size_byte, second_size_byte])?
        } else {
            packet.push(first_size_byte);

            // Decode the size of the packet
            VarU16::decode(vec![first_size_byte])?
        }
        .into_inner() as usize;

        // Read the rest of the packet
        let mut payload = vec![0; size];
        self.stream.reader.read_exact(&mut payload).await?;

        // Completely fill the packet
        packet.extend(payload);

        trace!("received packet: {:x?}", packet);

        // Push the packet to the incoming packets buffer
        self.incoming_packets.push(RawPacket::new(packet));

        Ok(())
    }
}

impl Connection for DaemonConnection {
    type Error = ConnectionError;

    fn connection_type(&self) -> vex_v5_serial::connection::ConnectionType {
        todo!()
    }

    async fn send_packet(
        &mut self,
        packet: impl vex_v5_serial::encode::Encode,
    ) -> Result<(), Self::Error> {
        // Encode the packet
        let encoded = packet.encode()?;

        trace!("sending packet: {:x?}", encoded);

        // Write the packet to the serial port
        self.stream.writer.write_all(&encoded).await?;
        self.stream.writer.flush().await?;

        Ok(())
    }

    async fn receive_packet<P: Decode + CheckHeader>(
        &mut self,
        timeout: Duration,
    ) -> Result<P, Self::Error> {
        // Return an error if the right packet is not received within the timeout
        select! {
            result = async {
                loop {
                    for packet in self.incoming_packets.iter_mut() {
                        if packet.check_header::<P>() {
                            match packet.decode_and_use::<P>() {
                                Ok(decoded) => {
                                    trim_packets(&mut self.incoming_packets);
                                    return Ok(decoded);
                                }
                                Err(e) => {
                                    error!("Failed to decode packet with valid header: {}", e);
                                    packet.used = true;
                                    return Err(e.into());
                                }
                            }
                        }
                    }
                    trim_packets(&mut self.incoming_packets);
                    self.receive_one_packet().await?;
                }
            } => result,
            _ = sleep(timeout) => Err(ConnectionError::Timeout)
        }
    }

    async fn read_user(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }

    async fn write_user(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}
