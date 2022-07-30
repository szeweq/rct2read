use std::io::{self, Read, Seek};

pub trait DSer {
    fn read_u64(&mut self) -> io::Result<u64>;
    fn read_u32(&mut self) -> io::Result<u32>;
    fn read_u16(&mut self) -> io::Result<u16>;
    fn read_u8(&mut self) -> io::Result<u8>;
    fn read_bytes(&mut self, sz: usize) -> io::Result<Vec<u8>>;
    fn read_dser<T>(&mut self) -> io::Result<T> where T: DeSerializable;
}

impl <S: Read + Seek> DSer for S {
    fn read_u64(&mut self) -> io::Result<u64> {
        let mut v = [0u8; 8];
        self.read_exact(&mut v)?;
        Ok(u64::from_le_bytes(v))
    }
    fn read_u32(&mut self) -> io::Result<u32> {
        let mut v = [0u8; 4];
        self.read_exact(&mut v)?;
        Ok(u32::from_le_bytes(v))
    }
    fn read_u16(&mut self) -> io::Result<u16> {
        let mut v = [0u8; 2];
        self.read_exact(&mut v)?;
        Ok(u16::from_le_bytes(v))
    }
    fn read_u8(&mut self) -> io::Result<u8> {
        let mut v = [0u8; 1];
        self.read_exact(&mut v)?;
        Ok(v[0])
    }
    fn read_bytes(&mut self, sz: usize) -> io::Result<Vec<u8>> {
        let mut v = Vec::with_capacity(sz);
        self.read_exact(&mut v)?;
        Ok(v)
    }
    fn read_dser<T>(&mut self) -> io::Result<T> where T: DeSerializable {
        let mut x = unsafe {std::mem::zeroed::<T>()};
        x.from_dser(self)?;
        Ok(x)
    }
}

pub trait DeSerializable {
    fn from_dser<S>(&mut self, ds: &mut S) -> io::Result<()> where S: DSer;
}

pub fn u32_from_slice(b: &[u8], at: usize) -> u32 {
    let mut v = [0u8; 4];
    v.copy_from_slice(&b[at..at+4]);
    u32::from_le_bytes(v)
}
pub fn u16_from_slice(b: &[u8], at: usize) -> u16 {
    let mut v = [0u8; 2];
    v.copy_from_slice(&b[at..at+2]);
    u16::from_le_bytes(v)
}