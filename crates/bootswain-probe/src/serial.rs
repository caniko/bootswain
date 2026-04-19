use anyhow::{Context, Result};
use serialport::{ClearBuffer, SerialPort};
use std::io::{self, Read, Write};
use std::time::Duration;

pub trait SerialIo {
    fn clear(&mut self) -> io::Result<()>;
    fn read_chunk(&mut self, buffer: &mut [u8]) -> io::Result<usize>;
    fn write_all(&mut self, buffer: &[u8]) -> io::Result<()>;
    fn flush(&mut self) -> io::Result<()>;
}

pub struct RealSerial {
    inner: Box<dyn SerialPort>,
}

impl RealSerial {
    pub fn open(port: &str, baud: u32) -> Result<Self> {
        let inner = serialport::new(port, baud)
            .timeout(Duration::from_millis(200))
            .open()
            .with_context(|| format!("failed to open serial port {port} at {baud} baud"))?;

        Ok(Self { inner })
    }
}

impl SerialIo for RealSerial {
    fn clear(&mut self) -> io::Result<()> {
        self.inner
            .clear(ClearBuffer::All)
            .map_err(|error| io::Error::other(error.to_string()))
    }

    fn read_chunk(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(buffer) {
            Ok(read) => Ok(read),
            Err(error) if error.kind() == io::ErrorKind::TimedOut => Ok(0),
            Err(error) => Err(error),
        }
    }

    fn write_all(&mut self, buffer: &[u8]) -> io::Result<()> {
        self.inner.write_all(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
