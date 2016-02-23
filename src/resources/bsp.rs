#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Vertex {
    pub x : i32,
    pub y : i32
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LineDef {
    pub v : [u32; 2],
    pub flags : u32,
    pub special_type : u32,
    pub sector_tag : u32,
    pub side : [u32; 2]
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SideDef {
    pub x_offset : i32,
    pub y_offset : i32,
    pub sector : u32
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Sector {
    pub floor_height : i32,
    pub ceiling_height : i32,
    pub light_level : u32,
    pub sector_type : u32,
    pub tag : u32
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Subsector {
    pub num_segs : u32,
    pub first_seg : u32
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LineSegment {
    pub v : [u32; 2],
    pub angle : u16,
    pub side : u16,
    pub line : u32,
    pub offset : i32
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Bounds {
    pub left : i32,
    pub top : i32,
    pub right: i32,
    pub bottom : i32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Node {
    pub x : i32,
    pub y : i32,
    pub dx : i32,
    pub dy : i32,
    pub bounds : [Bounds; 2],
    pub child : [u32; 2],
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Map {
    pub name : String,

    // Original level data
    pub lines : Vec<LineDef>,
    pub sides : Vec<SideDef>,
    pub sectors : Vec<Sector>,

    // Processed BSP data
    pub subsectors : Vec<Subsector>,
    pub segs : Vec<LineSegment>,
    pub nodes : Vec<Node>,

    // Misc
    pub vertices : Vec<Vertex>
}

