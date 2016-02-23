use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::SeekFrom;
use std::io::Seek;
use std::io::Read;
use byteorder::ReadBytesExt;
use byteorder::LittleEndian;
use byteorder::Error as ByteOrderError;
use resources::bsp;

macro_rules! try_io {
    ($e : expr) => {
        match $e {
            Ok(value) => value,
            Err(error) => return Err(WadError::IoFailure(error))
        }
    }
}

macro_rules! try_bo {
    ($e : expr) => {
        match $e {
            Ok(value) => value,
            Err(ByteOrderError::Io(error)) => return Err(WadError::IoFailure(error)),
            _ => panic!("UnexpectedEOF")
        }
    }
}

pub type WadResult<T> = Result<T, WadError>;

#[derive(Debug)]
pub enum WadError {
    IoFailure(io::Error),
    LumpMissing(&'static str),
    InvalidLump(&'static str)
}

struct LumpHeader {
    pos : u64,
    size : u64,
    name : [u8; 8]
}

struct LumpReader {
    pos : u64,
    lumps_left : u32
}

impl LumpReader {
    fn new(pos : u64, num_lumps : u32) -> LumpReader {
        LumpReader {
            pos: pos,
            lumps_left: num_lumps
        }
    }

    fn get(&self, reader : &mut BufReader<&mut File>) -> WadResult<Option<LumpHeader>>
    {
        if self.lumps_left == 0 {
            return Ok(None);
        }

        try_io!(reader.seek(SeekFrom::Start(self.pos)));

        let mut header = LumpHeader {
            pos: try_bo!(reader.read_u32::<LittleEndian>()) as u64,
            size: try_bo!(reader.read_u32::<LittleEndian>()) as u64,
            name: [0; 8]
        };

        try_io!(reader.read_exact(&mut header.name[..]));

        Ok(Some(header))
    }

    fn next(&mut self) {
        if self.lumps_left == 0 {
            return;
        }

        self.pos += 16;
        self.lumps_left -= 1;
    }
}

pub struct WadPackage {
    maps : Vec<bsp::Map>,
}

impl WadPackage {
    pub fn new(file : &mut File) -> WadResult<WadPackage> {
        try_io!(file.seek(SeekFrom::Start(0)));
        let mut reader = BufReader::new(file);

        // Lets trust that this is validated before.
        let mut signature = [0u8; 4];
        try_io!(reader.read_exact(&mut signature[..]));

        // Create the lump reader
        let mut lump_reader = {
            let num_lumps = try_bo!(reader.read_u32::<LittleEndian>());
            let lump_pos = try_bo!(reader.read_u32::<LittleEndian>()) as u64;
            LumpReader::new(lump_pos, num_lumps)
        };

        let mut package = WadPackage {
            maps: Vec::<bsp::Map>::new()
        };

        while let Some(lump) = try!(lump_reader.get(&mut reader)) {
            if is_map_lump(&lump) {
                package.maps.push(try!(read_map(lump.name, &mut reader, &mut lump_reader)));
            } else {
                lump_reader.next();
            }
        }

        Ok(package)
    }

    pub fn get_maps(&self) -> &[bsp::Map] {
        &self.maps[..]
    }
}

fn is_map_lump(lump : &LumpHeader) -> bool {
    if lump.size != 0 {
        return false;
    }

    // Doom 2 level format.
    if lump.name[0] == 'M' as u8 && lump.name[1] == 'A' as u8 && lump.name[2] == 'P' as u8 {
        if lump.name[3] < '0' as u8 || lump.name[3] > '9' as u8 {
            return false;
        }

        let mut map_num = lump.name[3] - '0' as u8;

        if lump.name[4] >= '0' as u8 || lump.name[4] <= '9' as u8 {
            map_num *= 10u8;
            map_num += lump.name[4] - '0' as u8
        } else if lump.name[4] != 0 {
            return false;
        }

        if map_num > 32u8 {
            return false;
        }

        return true;
    }

    if lump.name[0] == 'E' as u8 && lump.name[2] == 'M' as u8 {
        if lump.name[1] < '0' as u8 || lump.name[1] > '4' as u8 {
            return false;
        }
        if lump.name[3] < '0' as u8 || lump.name[3] > '9' as u8 {
            return false;
        }

        return true;
    }

    return false;
}

fn read_map(name_bytes : [u8; 8], reader : &mut BufReader<&mut File>, lump_reader : &mut LumpReader) -> WadResult<bsp::Map> {
    lump_reader.next();

    let mut name = String::new();
    for c in &name_bytes {
        if *c == 0 {
            break;
        }
        name.push(*c as char);
    }

    let mut level = bsp::Map {
        name: name,
        lines: Vec::<bsp::LineDef>::new(),
        sides: Vec::<bsp::SideDef>::new(),
        sectors: Vec::<bsp::Sector>::new(),
        subsectors: Vec::<bsp::Subsector>::new(),
        segs: Vec::<bsp::LineSegment>::new(),
        nodes: Vec::<bsp::Node>::new(),
        vertices: Vec::<bsp::Vertex>::new()
    };

    let mut tmp_buf = Vec::<u8>::new();

    try!(read_lump(false, reader, lump_reader, &mut tmp_buf, "THINGS", |_| {
        // TODO
        Ok(())
    }));

    try!(read_lump(true, reader, lump_reader, &mut tmp_buf, "LINEDEFS", |data| {
        let num = data.len() / 14;
        if data.len() % 14 != 0 {
            return Err(WadError::InvalidLump("LINEDEFS"));
        }

        level.lines.reserve(num);
        let mut reader = BufReader::new(&data[..]);

        for _ in 0..num {
            let v0 = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let v1 = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let flags = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let special_type = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let sector_tag = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let s0 = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let s1 = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            level.lines.push(bsp::LineDef {
                v: [v0, v1],
                flags: flags,
                special_type: special_type,
                sector_tag: sector_tag,
                side: [s0, s1]
            });
        }
        Ok(())
    }));

    try!(read_lump(true, reader, lump_reader, &mut tmp_buf, "SIDEDEFS", |data| {
        let num = data.len() / 30;
        if data.len() % 30 != 0 {
            return Err(WadError::InvalidLump("SIDEDEFS"));
        }

        level.sides.reserve(num);
        let mut reader = BufReader::new(&data[..]);

        for _ in 0..num {
            let x_offset = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let y_offset = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let mut upper_tex = [0u8; 8];
            try_io!(reader.read_exact(&mut upper_tex));
            let mut lower_tex = [0u8; 8];
            try_io!(reader.read_exact(&mut lower_tex));
            let mut mid_tex = [0u8; 8];
            try_io!(reader.read_exact(&mut mid_tex));
            let sector = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            level.sides.push(bsp::SideDef {
                x_offset: x_offset,
                y_offset: y_offset,
                sector: sector,
            });
        }
        Ok(())
    }));

    try!(read_lump(true, reader, lump_reader, &mut tmp_buf, "VERTEXES", |data| {
        let num = data.len() / 4;
        if data.len() % 4 != 0 {
            return Err(WadError::InvalidLump("VERTEXES"));
        }

        level.lines.reserve(num);
        let mut reader = BufReader::new(&data[..]);

        for _ in 0..num {
            let x = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let y = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            level.vertices.push(bsp::Vertex {
                x: x,
                y: y
            });
        }
        Ok(())
    }));

    try!(read_lump(true, reader, lump_reader, &mut tmp_buf, "SEGS", |data| {
        let num = data.len() / 12;
        if data.len() % 12 != 0 {
            return Err(WadError::InvalidLump("SEGS"));
        }

        level.lines.reserve(num);
        let mut reader = BufReader::new(&data[..]);

        for _ in 0..num {
            let v0 = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let v1 = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let angle = try_bo!(reader.read_u16::<LittleEndian>());
            let line = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let side = try_bo!(reader.read_u16::<LittleEndian>());
            let offset = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            level.segs.push(bsp::LineSegment {
                v: [v0, v1],
                angle: angle,
                line: line,
                side: side,
                offset: offset
            });
        }
        Ok(())
    }));

    try!(read_lump(true, reader, lump_reader, &mut tmp_buf, "SSECTORS", |data| {
        let num = data.len() / 4;
        if data.len() % 4 != 0 {
            return Err(WadError::InvalidLump("SSECTORS"));
        }

        level.lines.reserve(num);
        let mut reader = BufReader::new(&data[..]);

        for _ in 0..num {
            let num_segs = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let first_seg = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            level.subsectors.push(bsp::Subsector {
                num_segs: num_segs,
                first_seg: first_seg
            });
        }
        Ok(())
    }));

    try!(read_lump(true, reader, lump_reader, &mut tmp_buf, "NODES", |data| {
        let num = data.len() / 28;
        if data.len() % 28 != 0 {
            return Err(WadError::InvalidLump("NODES"));
        }

        level.lines.reserve(num);
        let mut reader = BufReader::new(&data[..]);

        for _ in 0..num {
            let x = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let y = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let dx = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let dy = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let bounds0_top = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let bounds0_bottom = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let bounds0_left = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let bounds0_right = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let bounds1_top = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let bounds1_bottom = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let bounds1_left = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let bounds1_right = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let mut child0 = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let mut child1 = try_bo!(reader.read_u16::<LittleEndian>()) as u32;

            if child0 & 0x8000u32 != 0u32 {
                child0 = (child0 & 0x7FFFu32) | 0x80000000u32;
            }

            if child1 & 0x8000u32 != 0u32 {
                child1 = (child1 & 0x7FFFu32) | 0x80000000u32;
            }

            level.nodes.push(bsp::Node {
                x: x,
                y: y,
                dx: dx,
                dy: dy,
                bounds: [bsp::Bounds {
                    top: bounds0_top,
                    bottom: bounds0_bottom,
                    left: bounds0_left,
                    right: bounds0_right,
                },
                bsp::Bounds {
                    top: bounds1_top,
                    bottom: bounds1_bottom,
                    left: bounds1_left,
                    right: bounds1_right,
                }],
                child: [child0, child1]
            });
        }
        Ok(())
    }));

    try!(read_lump(true, reader, lump_reader, &mut tmp_buf, "SECTORS", |data| {
        let num = data.len() / 26;
        if data.len() % 26 != 0 {
            return Err(WadError::InvalidLump("SECTORS"));
        }

        level.lines.reserve(num);
        let mut reader = BufReader::new(&data[..]);

        for _ in 0..num {
            let floor_height = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let ceiling_height = (try_bo!(reader.read_i16::<LittleEndian>()) as i32) << 16;
            let mut floor_tex = [0u8; 8];
            try_io!(reader.read_exact(&mut floor_tex));
            let mut ceiling_tex = [0u8; 8];
            try_io!(reader.read_exact(&mut ceiling_tex));
            let light_level = (try_bo!(reader.read_u16::<LittleEndian>()) as u32) << 16;
            let sector_type = try_bo!(reader.read_u16::<LittleEndian>()) as u32;
            let tag = try_bo!(reader.read_u16::<LittleEndian>()) as u32;

            level.sectors.push(bsp::Sector {
                floor_height: floor_height,
                ceiling_height: ceiling_height,
                light_level: light_level,
                sector_type: sector_type,
                tag: tag
            });
        }
        Ok(())
    }));

    Ok(level)
}

fn read_lump<F>(mandatory : bool, reader : &mut BufReader<&mut File>, lump_reader : &mut LumpReader,
    tmp_buf : &mut Vec<u8>, name : &'static str, f : F) -> WadResult<()>
    where F : FnOnce(&Vec<u8>) -> WadResult<()> {

    assert!(name.len() <= 8);
    if let Some(lump) = try!(lump_reader.get(reader)) {
        let mut b_iter = lump.name.iter();

        let mut lump_found = true;
        for a in name.bytes() {
            if let Some(&b) = b_iter.next() {
                if b == 0 {
                    break;
                } else if a != b {
                    lump_found = false;
                    break;
                }
            } else {
                break;
            }
        }

        if lump_found {
            try_io!(reader.seek(SeekFrom::Start(lump.pos)));
            tmp_buf.resize(lump.size as usize, 0u8);
            try_io!(reader.read_exact(&mut tmp_buf[..]));

            try!(f(tmp_buf));
            lump_reader.next();
            return Ok(());
        }
    }

    if mandatory {
        Err(WadError::LumpMissing(name))
    } else {
        Ok(())
    }
}
