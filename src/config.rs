pub const CHANNELS: usize = 1;
pub const DIES: usize = 4;
pub const PLANES: usize = 4;
pub const BLOCKS: usize = 4096;
pub const PAGES: usize = 2048;

pub const PAGE_SIZE: usize = 4096;

pub const FREE_BLOCKS: usize = 3;

pub const ALL_PLANES: usize = CHANNELS * DIES * PLANES;
pub const ALL_BLOCKS: usize = ALL_PLANES * BLOCKS;
pub const ALL_PAGES: usize = ALL_BLOCKS * PAGES;

pub const CAPACITY: usize = ALL_PAGES * PAGE_SIZE;
