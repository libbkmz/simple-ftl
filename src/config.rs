pub type BaseType = usize;

pub type Addr = BaseType;
pub type PageId = BaseType;
pub type BlockId = BaseType;
pub type Counter = BaseType;

pub const CHANNELS: BaseType = 1;
pub const DIES: BaseType = 1;
pub const PLANES: BaseType = 4;
pub const BLOCKS: BaseType = 4096;
pub const PAGES_PER_BLOCK: BaseType = 1028;

pub const PAGE_SIZE: BaseType = 4096;

pub const FREE_BLOCKS: BaseType = 5;

pub const ALL_DIES: BaseType = CHANNELS * DIES;
pub const ALL_PLANES: BaseType = ALL_DIES * PLANES;
pub const ALL_BLOCKS: BaseType = ALL_PLANES * BLOCKS;
pub const ALL_PAGES: BaseType = ALL_BLOCKS * PAGES_PER_BLOCK as BaseType;

pub const CAPACITY: usize = ALL_PAGES as usize * PAGE_SIZE as usize;

pub const INVALID_PAGE_ID: PageId = PageId::MAX;
