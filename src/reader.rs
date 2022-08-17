use crate::Result;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};

pub struct BufReaderWithPos<R: Read + Seek> {
    pub reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> BufReaderWithPos<R> {
    pub fn new(mut r: R, pos: u64) -> Result<Self> {
        r.seek(SeekFrom::Current(pos.try_into().unwrap()))?;

        Ok(Self {
            reader: BufReader::new(r),
            pos,
        })
    }
}

impl<R: Read + Seek> Read for BufReaderWithPos<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.pos += len as u64;
        Ok(len)
    }
}

impl<R: Read + Seek> Seek for BufReaderWithPos<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.pos = self.reader.seek(pos)?;
        Ok(self.pos)
    }
}

#[derive(Debug)]
pub struct BufWriterWithPos<W: Write + Seek> {
    writer: BufWriter<W>,
    pub pos: u64,
}

impl<W: Write + Seek> BufWriterWithPos<W> {
    pub fn new(mut w: W) -> Result<Self> {
        let pos = w.seek(SeekFrom::End(0))?;

        Ok(Self {
            writer: BufWriter::new(w),
            pos,
        })
    }
}

impl<W: Write + Seek> Write for BufWriterWithPos<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for BufWriterWithPos<W> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}
