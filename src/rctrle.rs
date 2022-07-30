use std::io::{self, Read, Bytes};

enum RunState {
    None, Copy(usize), Repeat(usize, u8)
}

pub struct Reader<R> {
    buf: Bytes<R>,
    state: RunState
}

impl<R: Read> Reader<R> {
    pub fn new(r: R) -> Reader<R> {
        Reader{buf: r.bytes(), state: RunState::None}
    }
    fn read_run(&mut self) {
        if let RunState::None = self.state {
            if let Some(Ok(b)) = self.buf.next() {
                let z = b as i8;
                if z > 0 {
                    self.state = RunState::Copy((z as usize) + 1);
                } else {
                    let c = self.buf.next().unwrap().unwrap();
                    self.state = RunState::Repeat(((-z) as usize) + 1, c);
                }
            }   
        }
    }
}
impl<R: Read> Read for Reader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut br = 0;

        for slot in buf {
            self.read_run();
            match self.state {
                RunState::Copy(n) => {
                    match self.buf.next() {
                        Some(Ok(b)) => {
                            *slot = b
                        }
                        _ => {
                            break
                        }
                    }
                    if n == 1 {
                        self.state = RunState::None;
                    } else {
                        self.state = RunState::Copy(n - 1);
                    }
                },
                RunState::Repeat(n, c) => {
                    *slot = c;
                    if n == 1 {
                        self.state = RunState::None;
                    } else {
                        self.state = RunState::Repeat(n - 1, c)
                    }
                }
                RunState::None => {
                    break
                }
            }
            br += 1;
        }
        Ok(br)
    }
}

pub fn rotate_bytes(b: &mut [u8]) {
    let mut i = 1;
    for x in b {
        *x = x.rotate_right(i);
        i += 2;
        if i > 7 {
            i = 1;
        }
    }
}

pub fn decompress(b: &mut [u8]) -> Vec<u8> {
    let mut i = 0;
    let mut v = Vec::new();
    while i < b.len() {
        let x = b[i];
        if x == 0xFF {
            v.push(b[i+1]);
            i += 2;
        } else {
            let ln = i32::from(x & 7) + 1;
            let of = i32::from(x >> 3) - 32;
            let sx = (v.len() as i32) + of;
            for nx in sx..(sx+ln) {
                v.push(v[nx as usize]);
            }
            i += 1;
        }
    }
    v
}