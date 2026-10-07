use std::io;

use futures::prelude::*;
use libp2p::StreamProtocol;
use libp2p::request_response::Codec;

pub(crate) const PROTOCOL: StreamProtocol = StreamProtocol::new("/kin/msg/1");

/// Maior mensagem aceita (já cifrada), para um peer hostil não nos fazer alocar sem limite.
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;

/// Mensagem opaca com prefixo de tamanho (u32 big-endian); a resposta é um ack de 1 byte.
#[derive(Clone, Default)]
pub(crate) struct MessageCodec;

impl Codec for MessageCodec {
    type Protocol = StreamProtocol;
    type Request = Vec<u8>;
    type Response = ();

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Vec<u8>>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len = [0u8; 4];
        io.read_exact(&mut len).await?;
        let len = u32::from_be_bytes(len) as usize;
        if len > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "mensagem grande demais",
            ));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        Ok(buf)
    }

    async fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<()>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut ack = [0u8; 1];
        io.read_exact(&mut ack).await?;
        Ok(())
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Vec<u8>,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        if req.len() > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "mensagem grande demais",
            ));
        }
        io.write_all(&(req.len() as u32).to_be_bytes()).await?;
        io.write_all(&req).await?;
        io.close().await
    }

    async fn write_response<T>(&mut self, _: &Self::Protocol, io: &mut T, _: ()) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        io.write_all(&[1]).await?;
        io.close().await
    }
}
