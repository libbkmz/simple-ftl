
#[cfg(test)]
use std::{println as info, println as warn, println as error, println as trace, println as debug };
use std::collections::VecDeque;
use byte_unit::Byte;

#[cfg(not(test))]
use log::*;

// mod config;
use crate::config::*;
type Addr = usize;
type PageId = usize;
type BlockId = usize;
type Counter = usize;

pub struct Ftl {
    l2p: Vec<L2PValue>,
    // p2l: [u32; ALL_PAGES],
    logical_size: Addr,

    host_open_block: Block,
    host_gc_open_block: Block,

    free_blocks: VecDeque<Block>,
    full_blocks: VecDeque<Block>
}

#[derive(Clone)] // for Vec resize
pub enum L2PValue {
    Invalid,
    Valid(Addr),
}



#[derive(Clone)]
pub struct Block {
    block_id: Addr,
    valid_counter: Counter,
    cursor: PageId, // points to the next addr
    erase_counter: Counter,
}

impl Block {
    pub fn new(block_id: Addr) -> Self {
        Block {
            block_id,
            valid_counter: 0,
            cursor: 0,
            erase_counter: 0
        }
    }

    fn invalid_counter(&self) -> Counter {
        PAGES - self.valid_counter
    }

    fn full(&self) -> bool {
        self.valid_counter == PAGES
    }

    fn write_one_page(&mut self) {
        debug_assert!(self.valid_counter <= PAGES, "write more pages than block has");

        self.cursor += 1;
        self.valid_counter += 1;
    }

    fn erase(&mut self){
        self.cursor = 0;
        self.valid_counter = 0;
        self.erase_counter += 1;
    }

}

impl Ftl {
    pub fn default() -> Self {
        Ftl {
            l2p: Vec::new(),
            logical_size: 0,
            host_open_block: Block::new(0),
            host_gc_open_block: Block::new(1),
            free_blocks: VecDeque::from((2..BLOCKS).map(|x| Block::new(x)).collect::<Vec<_>>()),
            full_blocks: VecDeque::with_capacity(BLOCKS),
        }
    }
    pub fn new_with_op(op: f64) -> Self {
        use byte_unit::Byte;

        let mut out = Ftl::default();
        let op_pages = (ALL_PAGES as f64 * (op / 100.)).trunc() as Addr;

        trace!("ALL_PAGES: {}", ALL_PAGES);
        trace!("op_pages: {}", op_pages);
        trace!("Physical Capacity: {} bytes, {}", ALL_PAGES * PAGE_SIZE, Byte::from(ALL_PAGES * PAGE_SIZE).get_appropriate_unit(true).to_string());
        trace!("User Capacity: {} bytes, {}", (ALL_PAGES - op_pages) * PAGE_SIZE, Byte::from((ALL_PAGES - op_pages) * PAGE_SIZE).get_appropriate_unit(true).to_string());


        out.logical_size = ALL_PAGES - op_pages;
        out.l2p.resize(out.logical_size, L2PValue::Invalid);

        out
    }

    fn gc(&mut self) {
        // @TODO: find better victim block, based on valid/invalid pages
        let mut victim_block = self.full_blocks.pop_front().unwrap();
        // @TODO: add real page address usage
        victim_block.erase();

        // FIXME: remove clone
        self.free_blocks.push_back(victim_block.clone());
    }

    // @TODO: optimise writes for up to the size of the block to speedup
    pub fn write(&mut self, number_of_pages: usize) -> Result<bool, &'static str> {
        debug_assert_ne!(number_of_pages, 0, "zero writes not supported");

        for idx in 0..number_of_pages {
            if self.host_open_block.full() {
                // FIXME: remove clone
                self.full_blocks.push_back(self.host_open_block.clone());
                self.host_open_block = self.free_blocks.pop_front().unwrap();
            }
            if self.free_blocks.len() < FREE_BLOCKS {
                self.gc();
            }

            // debug!("Host OpenBlock addr: {}", self.host_open_block.block_id);
            debug_assert!(!self.host_open_block.full(), "host_open_block is full before write");

            // trace!("Write {}", idx);
            self.host_open_block.write_one_page();

        }

        Ok(true)
    }
}
