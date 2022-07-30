use std::fs::File;
use std::io::Read;
use crate::rctrle;
use crate::util::{u16_from_slice, u32_from_slice};

pub fn read_td6_file(mut f: &File) {
    let mut vf = Vec::new();
    f.read_to_end(&mut vf).unwrap();
    //println!("{} - {:?}", vf.len(), vf);
    let checksum = &vf[vf.len()-4..];
    println!("CHECKSUM: {:x?}", checksum);
    let mut rd = rctrle::Reader::new(&vf[0..vf.len()-4]);
    let mut v = Vec::new();
    rd.read_to_end(&mut v).unwrap();
    //println!("{} - {:?}", v.len(), v);
    let track_type = v[0];
    println!("Track type: {}", track_type);
    let air_time = i32::from(v[0x4A]) * 4;
    println!("Air time: {}", air_time);
    let num_of_trains = v[0x4C];
    println!("Number of trains: {}", num_of_trains);
    let cars_per_train = v[0x4D];
    println!("Cars per train: {}", cars_per_train);
    let speed = v[0x50];
    println!("Speed: {}", speed);
    let excitement = f32::from(v[0x5B]) / 10.0;
    let intensity = f32::from(v[0x5C]) / 10.0;
    let nausea = f32::from(v[0x5D]) / 10.0;
    println!("Excitement: {}; Intensity: {}; Nausea: {}", excitement, intensity, nausea);
    let mut i = 0;
    loop {
        let b1 = v[0xA3+2*i];
        if b1 == 0xFF {
            break;
        }
        let b2 = v[0xA3+2*i+1];
        println!("Track [{}; q: {:08b}]", segment_name(b1), b2);
        i += 1;
    }
}

pub fn read_sv6_file(f: &File) {
    // Read header
    println!("Header...");
    let hv = read_sv6_chunk(&f);
    let co = u16_from_slice(&hv, 2);

    // How to read custom import objects?
    if co > 0 {
        println!("Can't read Custom Objects yet... Sorry.");
        return;
    }

    // Read available items
    println!("Items...");
    read_sv6_chunk(&f);

    // Read date
    println!("Flags 1...");
    let vd = read_sv6_chunk(&f);
    let month = u16_from_slice(&vd, 0);
    let day = u32::from(u16_from_slice(&vd, 2)) * 16 / 0x8421 + 1;
    println!("Day: {}; Month: {}; Year: {}", day, month % 8, month / 8);

    // Read game map
    println!("Map...");
    read_sv6_chunk(&f);

    // Read game data
    println!("Game data...");
    let vx = read_sv6_chunk(&f);
    println!("Initial cash: {}", u32_from_slice(&vx, 0x27_1024));
    println!("Loan: {}", u32_from_slice(&vx, 0x27_1028));
    println!("Entrance fee: {}", u32_from_slice(&vx, 0x27_1030));
    let guests_in_park = u16_from_slice(&vx, 0x27_148C);
    println!("Guests in park: {}", guests_in_park);
    let park_rating = u16_from_slice(&vx, 0x27_18F8);
    println!("Park rating: {}", park_rating);
    println!("Real cash: {}", decrypt_money(u32_from_slice(&vx, 0x27_2440)));

    read_sv6_rides(&vx[0x27_C540..0x2A_22E0], month);
}

fn read_sv6_chunk(mut f: &File) -> Vec<u8> {
    let mut cb: [u8; 5] = [0; 5];
    f.read_exact(&mut cb[..]).unwrap();
    let sz = u32_from_slice(&cb, 1) as usize;
    let mut ch = vec![0u8; sz];
    f.read_exact(&mut ch).unwrap();
    match cb[0] {
        0 => {
            ch
        }
        1 => {
            let mut rd = rctrle::Reader::new(&ch[..]);
            let mut cd = Vec::new();
            rd.read_to_end(&mut cd).unwrap();
            cd
        },
        2 => {
            let mut rd = rctrle::Reader::new(&ch[..]);
            let mut cd = Vec::new();
            rd.read_to_end(&mut cd).unwrap();
            rctrle::decompress(&mut cd)
        }
        3 => {
            rctrle::rotate_bytes(&mut ch);
            ch
        },
        _ => {
            vec!()
        }
    }
}

pub fn read_sv6_rides(mut b: &[u8], pmon: u16) {
    let mut n = 0;
    while !b.is_empty() {
        if b[0] == 0xFF {
            return;
        }
        let excitement = u16_from_slice(b, 0x140);
        let intensity = u16_from_slice(b, 0x142);
        let nausea = u16_from_slice(b, 0x144);
        let constructed = u16_from_slice(b, 0x180);
        println!("+-= RIDE 0x{:X} =-", b[0]);
        println!("| Excitement: {}; Intensity: {}; Nausea: {}", excitement, intensity, nausea);
        println!("| Age (months): {}", pmon - constructed);
        println!("| Ticket price: {:.2} (suggested {:.2})", f32::from(u16_from_slice(b, 0x138)) / 10.0, calculate_price(b[0], excitement, intensity, nausea, pmon - constructed) / 10.0);
        println!("| Calculated: {:.2}", f64::from(calculate_price_orig(b[0], excitement, intensity, nausea, pmon - constructed)) / 10.0);
        println!(".");
        b = &b[608..];
        n += 1
    }
    println!("Number of rides: {}", n);
}

pub fn calculate_price(ride: u8, exc: u16, int: u16, nau: u16, age: u16) -> f64 {
    let (m_exc, m_int, m_nau) = ride_rating(ride);
    let m_age = match age {
        _ if age < 5 => 1.5,
        _ if age < 13 => 1.2,
        _ if age < 40 => 1.0,
        _ if age < 64 => 0.75,
        _ if age < 88 => 0.56,
        _ if age < 104 => 0.42,
        _ if age < 120 => 0.32,
        _ if age < 128 => 0.16,
        _ if age < 200 => 0.08,
        _ => 0.56
    };
    let bv = (i32::from(exc) * m_exc + i32::from(int) * m_int + i32::from(nau) * m_nau) / 1024;
    (m_age * f64::from(bv) * 2.0 - 1.0).floor().max(0.0)
}

pub fn calculate_price_orig(ride: u8, exc: u16, int: u16, nau: u16, age: u16) -> i32 {
    let (m_exc, m_int, m_nau) = ride_rating(ride);
    let (m_age, d_age) = match age {
        _ if age < 5 => (3, 2),
        _ if age < 13 => (6, 5),
        _ if age < 40 => (1, 1),
        _ if age < 64 => (3, 4),
        _ if age < 88 => (9, 16),
        _ if age < 104 => (27, 64),
        _ if age < 120 => (81, 256),
        _ if age < 128 => (81, 512),
        _ if age < 200 => (81, 1024),
        _ => (9, 16)
    };
    //let bv = (((i32::from(exc) * m_exc) * 32) >> 15) + (((i32::from(int) * m_int) * 32) >> 15) + (((i32::from(nau) * m_nau) * 32) >> 15);
    let bv = ((i32::from(exc) * m_exc) >> 10) + ((i32::from(int) * m_int) >> 10) + ((i32::from(nau) * m_nau) >> 10);
    (bv * m_age / d_age * 2 - 1).max(0)
}

pub fn decrypt_money(c: u32) -> u32 {
    (c ^ 0xF4EC_9621).rotate_left(13)
}

pub fn ride_rating(ride: u8) -> (i32, i32, i32) {
    const C_RATING: (i32, i32, i32) = (50, 30, 10);
    match ride {
        0..=4 => C_RATING,
        5 | 6 => (70, 6, -10),
        7 => C_RATING,
        8 => (70, 6, 0),
        9 => (50, 30, 30),
        0xA => C_RATING,
        0xB => (70, 10, 10),
        0xC => (50, 50, 10),
        0xD => C_RATING,
        0xE => (80, 10, 0),
        0xF..=0x11 => C_RATING,
        0x12 => (70, 10, 0),
        0x13 => C_RATING,
        0x14 => (50, 0, 0),
        0x15 => (50, 10, 0),
        0x16 => (120, 0, 0),
        0x17 => (80, 34, 6),
        0x18 => (72, 26, 6),
        0x19 => (40, 20, 0),
        0x1A | 0x1B => C_RATING,
        // Stalls (1C-20)
        0x21 => (50, 10, 0),
        // Buildings (22-24)
        0x25 => (60, 20, 10),
        0x26 => (24, 20, 10),
        0x27 => (20, 10, 0),
        0x28 => (24, 20, 10),
        0x29 => (12, 4, 4),
        0x2A => (44, 66, 10),
        0x2B => (80, 10, 0),
        0x2C => (52, 38, 10),
        // ATM (2D)
        0x2E => (40, 20, 10),
        0x2F => (20, 10, 0),
        // First Aid (30)
        0x31 => (20, 10, 0),
        0x32 => (70, 10, 10),
        0x33 => (52, 36, 10),
        0x34 => (52, 33, 8),
        0x35 => (48, 28, 7),
        0x36 => (50, 30, 30),
        0x37 | 0x39 => C_RATING,
        // NONE (38, 3A)
        0x3B => (30, 15, 25),
        0x3C => (80, 34, 6),
        0x3D => (70, 10, 10),
        0x3E => C_RATING,
        0x3F => (70, 6, -10),
        // NONE (40)
        0x41 => (48, 28, 7),
        0x42 | 0x43 => C_RATING,
        0x44 => (51, 32, 10),
        0x45 => (50, 50, 10),
        0x46 => (50, 25, 0),
        0x47 => (15, 8, 0),
        0x48 => (50, 10, 10),
        0x49 | 0x4A => C_RATING,
        0x4B => (44, 66, 10),
        0x4C => (50, 30, 30),
        0x4D => C_RATING,
        0x4E => (70, 6, 0),
        0x4F => (80, 34, 6),
        // NONE (50)
        0x51 => (50, 50, 0),
        // NONE (52-55)
        0x56 | 0x57 => C_RATING,
        0x58 => (60, 20, 10),
        0x5A => C_RATING,
        _ => (0, 0, 0)
    }
}

pub fn segment_name(n: u8) -> &'static str {
    match n {
        0x00 => "Flat",
        0x01 => "Station end",
        0x02 => "Station begin",
        0x03 => "Station inside",
        0x04 => "25↑",
        0x05 => "60↑",
        0x06 => "flat to 25↑",
        0x07 => "25↑ to 60↑",
        0x08 => "60↑ to 25↑",
        0x09 => "25↑ to flat",
        0x0A => "25↓",
        0x0B => "60↓",
        0x0C => "flat to 25↓",
        0x0D => "25↓ to 60↓",
        0x0E => "60↓ to 25↓",
        0x0F => "25↓ to flat",
        0x10 => "L ¼ D5",
        0x11 => "R ¼ D5",
        0x12 => "flat to L bank",
        0x13 => "flat to R bank",
        0x14 => "L bank to flat",
        0x15 => "R bank to flat",
        0x16 => "L ¼ D5 bank",
        0x17 => "R ¼ D5 bank",
        0x18 => "L bank to 25↑",
        0x19 => "R bank to 25↑",
        0x1A => "25↑ to L bank",
        0x1B => "25↑ to R bank",
        0x1C => "L bank to 25↓",
        0x1D => "R bank to 25↓",
        0x1E => "25↓ to L bank",
        0x1F => "25↓ to R bank",
        0x20 => "L bank",
        0x21 => "R bank",
        0x22 => "L ¼ D5 25↑",
        0x23 => "R ¼ D5 25↑",
        0x24 => "L ¼ D5 25↓",
        0x25 => "R ¼ D5 25↓",
        0x26 => "L S-bend",
        0x27 => "R S-bend",
        0x28 => "L vertical loop",
        0x29 => "R vertical loop",
        0x2A => "L ¼ D3",
        0x2B => "R ¼ D3",
        0x2C => "L ¼ D3 bank",
        0x2D => "R ¼ D3 bank",
        0x2E => "L ¼ D3 25↑",
        0x2F => "R ¼ D3 25↑",
        0x30 => "L ¼ D3 25↓",
        0x31 => "R ¼ D3 25↓",
        0x32 => "L ¼ D1",
        0x33 => "R ¼ D1",
        0x34 => "L twist ↓ to ↑",
        0x35 => "R twist ↓ to ↑",
        0x36 => "L twist ↑ to ↓",
        0x37 => "R twist ↑ to ↓",
        0x38 => "½ loop ↑",
        0x39 => "½ loop ↓",
        0x3A => "L corkscrew ↑",
        0x3B => "R corkscrew ↑",
        0x3C => "L corkscrew ↓",
        0x3D => "R corkscrew ↓",
        0x3E => "flat to 60↑",
        0x3F => "60↑ to flat",
        0x40 => "flat to 60↓",
        0x41 => "60↓ to flat",
        0x42 => "tower base",
        0x43 => "tower section",
        0x44 => "flat covered",
        0x45 => "25↑ covered",
        0x46 => "60↑ covered",
        0x47 => "flat to 25↑ covered",
        0x48 => "25↑ to 60↑ covered",
        0x49 => "60↑ to 25↑ covered",
        0x4A => "25↑ to flat covered",
        0x4B => "25↓ covered",
        0x4C => "60↓ covered",
        0x4D => "flat to 25↓ covered",
        0x4E => "25↓ to 60↓ covered",
        0x4F => "60↓ to 25↓ covered",
        0x50 => "25↓ to flat covered",
        0x51 => "L ¼ D5 covered",
        0x52 => "R ¼ D5 covered",
        0x53 => "L S-bend covered",
        0x54 => "R S-bend covered",
        0x55 => "L ¼ D3 covered",
        0x56 => "R ¼ D3 covered",
        0x57 => "L ½ banked helix ↑ small",
        0x58 => "R ½ banked helix ↑ small",
        0x59 => "L ½ banked helix ↓ small",
        0x5A => "R ½ banked helix ↓ small",
        0x5B => "L ½ banked helix ↑ large",
        0x5C => "R ½ banked helix ↑ large",
        0x5D => "L ½ banked helix ↓ large",
        0x5E => "R ½ banked helix ↓ large",
        0x5F => "L ¼ D1 60↑",
        0x60 => "R ¼ D1 60↑",
        0x61 => "L ¼ D1 60↓",
        0x62 => "R ¼ D1 60↓",
        0x63 => "brakes",
        0x64 => "booster RCT2: Rotation control toggle (Spinning Wild Mouse)",
        0x65 => "{reserved}RCT2: inverted 90↑ to flat quarter loop (multidim)",
        0x66 => "L ¼ banked helix large↑",
        0x67 => "R ¼ banked helix large↑",
        0x68 => "L ¼ banked helix large↓",
        0x69 => "R ¼ banked helix large↓",
        0x6A => "L ¼ helix large↑",
        0x6B => "R ¼ helix large↑",
        0x6C => "L ¼ helix large↓",
        0x6D => "R ¼ helix large↓",
        0x6E => "{ride base: 2 X 2} RCT2: 25↑ L banked",
        0x6F => "{ride base: 4 X 4} RCT2: 25↑ R banked",
        0x70 => "waterfall",
        0x71 => "rapids",
        0x72 => "on ride photo",
        0x73 => "{reserved}RCT2: 25↓ L banked",
        0x74 => "{ride base: 1 X 5}RCT2: 25↓ R banked",
        0x75 => "watersplash",
        0x76 => "{shop/stall} RCT2: flat to 60↑ - long base",
        0x77 => "{ride base: 1 X 2} RCT2: 60↑ to flat - long base",
        0x78 => "whirlpool",
        0x79 => "{info kiosk}RCT2: 60↓ to flat - long base",
        0x7A => "{ride base: 1 X 4}RCT2: flat to 60↓ - long base",
        0x7B => "{ride base: 3 X 3}RCT2: Cable Lift Hill",
        0x7C => "reverse whoa belly slope",
        0x7D => "reverse whoa belly vertical",
        0x7E => "90↑",
        0x7F => "90↓",
        0x80 => "60↑ to 90↑",
        0x81 => "90↓ to 60↓",
        0x82 => "90↑ to 60↑",
        0x83 => "60↓ to 90↓",
        0x84 => "brake for drop",
        0x85 => "L 1/8 OTD",
        0x86 => "R 1/8 OTD",
        0x87 => "L 1/8 DTO",
        0x88 => "R 1/8 DTO",
        0x89 => "L 1/8 bank OTD",
        0x8A => "R 1/8 bank OTD",
        0x8B => "L 1/8 bank DTO",
        0x8C => "R 1/8 bank DTO",
        0x8D => "Diag flat",
        0x8E => "Diag 25↑",
        0x8F => "Diag 60↑",
        0x90 => "Diag flat to 25↑",
        0x91 => "Diag 25↑ to 60↑",
        0x92 => "Diag 60↑ to 25↑",
        0x93 => "Diag 25↑ to flat",
        0x94 => "Diag 25↓",
        0x95 => "Diag60↓",
        0x96 => "Diag flat to 25↓",
        0x97 => "Diag 25↓ to 60↓",
        0x98 => "Diag 60↓ to 25↓",
        0x99 => "Diag 25↓ to flat",
        0x9A => "Diag flat to 60↑",
        0x9B => "Diag 60↑ to flat",
        0x9C => "Diag flat to 60↓",
        0x9D => "Diag 60↓ to flat",
        0x9E => "Diag flat to L bank",
        0x9F => "Diag flat to R bank",
        0xA0 => "Diag L bank to flat",
        0xA1 => "Diag R bank to flat",
        0xA2 => "Diag L bank to 25↑",
        0xA3 => "Diag R bank to 25↑",
        0xA4 => "Diag 25↑ to L bank",
        0xA5 => "Diag 25↑ to R bank",
        0xA6 => "Diag L bank to 25↓",
        0xA7 => "Diag R bank to 25↓",
        0xA8 => "Diag 25↓ to L bank",
        0xA9 => "Diag 25↓ to R bank",
        0xAA => "Diag L bank",
        0xAB => "Diag R bank",
        0xAC => "Log flume reverser",
        0xAD => "spinning tunnel",
        0xAE => "L barrel roll ↑ to ↓",
        0xAF => "R barrel roll ↑ to ↓",
        0xB0 => "L barrel roll ↓ to ↑",
        0xB1 => "R barrel roll ↓ to ↑",
        0xB2 => "L bank to L ¼ D3 25↑",
        0xB3 => "R bank to R ¼ D3 25↑",
        0xB4 => "L ¼ D3 25↓ to L bank",
        0xB5 => "R ¼ D3 25↓ to R bank",
        0xB6 => "powered lift",
        0xB7 => "L large ½ loop ↑",
        0xB8 => "R large ½ loop ↑",
        0xB9 => "R large ½ loop ↓",
        0xBA => "L large ½ loop ↓",
        0xBB => "L flyer twist ↑ to ↓",
        0xBC => "R flyer twist ↑ to ↓",
        0xBD => "L flyer twist ↓ to ↑",
        0xBE => "R flyer twist ↓ to ↑",
        0xBF => "flyer ½ loop ↑",
        0xC0 => "flyer ½ loop ↓",
        0xC1 => "L fly corkscrw ↑ to ↓",
        0xC2 => "R fly corkscrw ↑ to ↓",
        0xC3 => "L fly corkscrw ↓ to ↑",
        0xC4 => "R fly corkscrew ↓ to ↑",
        0xC5 => "heartline transfer up",
        0xC6 => "heartline transfer down",
        0xC7 => "L heartline roll",
        0xC8 => "R heartline roll",
        0xC9 => "mini golf hole A",
        0xCA => "mini golf hole B",
        0xCB => "mini golf hole C",
        0xCC => "mini golf hole D",
        0xCD => "RCT2: mini golf hole E",
        0xCE => "RCT2: inverted flat to 90↓ quarter loop (multidim)",
        0xCF => "RCT2: Quarter loop 90↑ to invert",
        0xD0 => "RCT2: Quarter loop invert to 90↓",
        0xD1 => "RCT2: L curved lift hill",
        0xD2 => "RCT2: R curved lift hill",
        0xD3 => "L reverser",
        0xD4 => "R reverser",
        0xD5 => "Air Thrust top cap",
        0xD6 => "Air Thrust Vertical down",
        0xD7 => "Air Thrust vertical down to level",
        0xD8 => "Block Brakes",
        0xD9 => "L ¼ D3 25↑ banked",
        0xDA => "R ¼ D3 25↑ banked",
        0xDB => "L ¼ D3 25↓ banked",
        0xDC => "R ¼ D3 25↓ banked",
        0xDD => "L ¼ D5 25↑ banked",
        0xDE => "R ¼ D5 25↑ banked",
        0xDF => "L ¼ D5 25↓ banked",
        0xE0 => "R ¼ D5 25↓ banked",
        0xE1 => "25↑ to L bank 25↑",
        0xE2 => "25↑ to R bank 25↑",
        0xE3 => "L bank 25↑ to 25↑",
        0xE4 => "R bank 25↑ to 25↑",
        0xE5 => "25↓ to L bank 25↓",
        0xE6 => "25↓ to R bank 25↓",
        0xE7 => "L bank 25↓ to 25↓",
        0xE8 => "R bank 25↓ to 25↓",
        0xE9 => "L bank to L bank 25↑",
        0xEA => "R bank to R bank 25↑",
        0xEB => "L bank 25↑ to L bank flat",
        0xEC => "R bank 25↑ to R bank flat",
        0xED => "L bank to L bank 25↓",
        0xEE => "R bank to R bank 25↓",
        0xEF => "L bank 25↓ to L bank flat",
        0xF0 => "R bank 25↓ to R bank flat",
        0xF1 => "flat to L bank 25↑",
        0xF2 => "flat to R bank 25↑",
        0xF3 => "L bank 25↑ to flat",
        0xF4 => "R bank 25↑ to flat",
        0xF5 => "flat to L bank 25↓",
        0xF6 => "flat to R bank 25↓",
        0xF7 => "L bank 25↓ to flat",
        0xF8 => "R bank 25↓ to flat",
        0xF9 => "L ¼ D1 90↑",
        0xFA => "R ¼ D1 90↑",
        0xFB => "L ¼ D1 90↓",
        0xFC => "R ¼ D1 90↓",
        0xFD => "90↑ to inverted flat quarter loop (multidim)",
        0xFE => "flat to 90↓ quarter loop (multidim)",
        _ => "UNK"
    }
}
