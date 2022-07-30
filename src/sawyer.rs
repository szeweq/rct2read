use std::io::{self, Read, Seek, SeekFrom};
use crate::util;
use crate::util::DSer;

#[derive(Copy, Clone)]
pub enum Encoding {
    None,
    RLE,
    RLECompressed,
    Rotate,
    Unknown
}

impl From<u8> for Encoding {
    fn from(n: u8) -> Self {
        use Encoding::*;
        match n {
            0 => None,
            1 => RLE,
            2 => RLECompressed,
            3 => Rotate,
            _ => Unknown
        }
    }
}

pub struct ChunkHeader {
    enc: Encoding,
    len: u32
}

impl util::DeSerializable for ChunkHeader {
    fn from_dser<S>(&mut self, ds: &mut S) -> io::Result<()> where S: util::DSer {
        self.enc = Encoding::from(ds.read_u8()?);
        self.len = ds.read_u32()?;
        Ok(())
    }
}

pub struct Chunk {
    data: Vec<u8>,
    enc: Encoding
}

impl Chunk {
    pub fn new(enc: Encoding, data: Vec<u8>) -> Self {
        Self{data, enc}
    }
    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
    pub fn encoding(&self) -> Encoding {
        self.enc
    }
}

pub struct ChunkReader<S: Read + Seek> {
    s: S
}

impl <S: Read + Seek> ChunkReader<S> {
    pub fn new(s: S) -> Self where S: Read + Seek {
        ChunkReader{s}
    }

    fn get_position(&mut self) -> io::Result<u64> {
        self.s.seek(SeekFrom::Current(0))
    }

    pub fn skip_chunk(&mut self) -> io::Result<()> {
        let header = self.s.read_dser::<ChunkHeader>()?;
        self.s.seek(SeekFrom::Current(header.len.into()))?;
        Ok(())
    }

    pub fn read_chunk(&mut self) -> io::Result<()> {
        //let orig_pos = self.get_position()?;
        let header = self.s.read_dser::<ChunkHeader>()?;
        self.s.read_bytes(header.len as usize)?;
        Ok(())
    }

    //pub fn read_chunk_to<T>(&mut self, d: *mut T) {}

    fn decode_chunk(&self, h: &ChunkHeader, from: &[u8], to: &mut Vec<u8>) {
        use Encoding::*;
        match h.enc {
            None => {
                to.copy_from_slice(from);
            },
            RLE => {
                self.decode_chunk_rle(from, to);
            },
            RLECompressed => {
                self.decode_chunk_rle_repeat(from, to);
            },
            Rotate => {
                self.decode_chunk_rotate(from, to);
            },
            _ => {}
        }
    }
    fn decode_chunk_rle_repeat(&self, from: &[u8], to: &mut Vec<u8>) {
        let mut imbuf = Vec::new();
        self.decode_chunk_rle(from, &mut imbuf);
        self.decode_chunk_repeat(&imbuf, to);
    }
    fn decode_chunk_rle(&self, from: &[u8], to: &mut Vec<u8>) {
        let mut i = 0;
        while i < from.len() {
            let x = from[i];
            if x & 128 != 0 {
                
            } else {
                to.extend_from_slice(&from[i+1..i+1+x as usize]);
                i += x as usize + 1;
            }
            i += 1;
        }
    }
    fn decode_chunk_repeat(&self, from: &[u8], to: &mut Vec<u8>) {
        let mut i = 0;
        while i < from.len() {
            let x = from[i];
            if x == 0xFF {
                to.push(from[i+1]);
                i += 2;
            } else {
                let ln = i32::from(x & 7) + 1;
                let sx = (to.len() as i32) + i32::from(x >> 3) - 32;
                let cto = to.clone();
                let iter= cto.iter().skip(sx as usize).take(ln as usize);//.collect_into(to);
                to.extend(iter);
                //for nx in sx..(sx+ln) {to.push(to[nx as usize]);}
                i += 1;
            }
        }
    }
    fn decode_chunk_rotate(&self, from: &[u8], to: &mut Vec<u8>) {
        let mut i = 1;
        for x in from {
            to.push(x.rotate_right(i));
            i = (i + 2) % 8;
        }
    }
}

pub fn validate_checksum<S>(mut s: S) -> bool where S: Read + Seek {
    let init_pos = s.seek(SeekFrom::Current(0)).unwrap();
    let mut sz = s.seek(SeekFrom::End(0)).unwrap() - init_pos;
    if sz < 8 {
        return false;
    }
    sz -= 4;
    s.seek(SeekFrom::Start(init_pos)).unwrap();
    let mut checksum = 0u32;
    while sz != 0 {
        let mut buf = [0u8; 4096];
        let bs = sz.min(4096);
        if s.read_exact(&mut buf[..]).is_err() {
            s.seek(SeekFrom::Start(init_pos)).unwrap();
            return false;
        }
        for &x in buf.iter() {
            checksum += u32::from(x);
        }
        sz -= bs;
    }
    let mut chbuf = [0u8; 4];
    s.read_exact(&mut chbuf[..]).unwrap();
    let fch = u32::from_le_bytes(chbuf);
    checksum == fch
}