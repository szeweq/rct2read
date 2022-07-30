mod rctrle;
mod rct;
mod sawyer;
mod s6;
mod util;

use std::fs::File;
use std::path::Path;

fn main() {
    let farg = match std::env::args().nth(1) {
        Some(n) => n,
        None => {
            println!("No file provided");
            return;
        }
    };
    let p = Path::new(farg.as_str());
    let f = File::open(p).unwrap();
    if let Some(x) = p.extension() {
        match x.to_str() {
            Some("td6") => {
                rct::read_td6_file(&f);
            },
            Some("sv6") => {
                rct::read_sv6_file(&f);
            },
            _ => {
                println!("Unsupported extension");
            }
        }
    }
    
}

