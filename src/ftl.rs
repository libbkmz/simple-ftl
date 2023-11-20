#[cfg(test)]
use std::{println as info, println as warn, println as error, println as trace, println as debug};

#[cfg(not(test))]
use log::*;

use crate::config::*;
use num_integer::div_rem;
use std::cell::UnsafeCell;
use std::collections::VecDeque;

pub struct Ftl {
    l2p: Vec<PageId>,
    logical_size: Addr, // or MAX_LBA

    host_open_block_idx: BlockId, // could be also RefCell for interior mutability
    host_gc_open_block_idx: BlockId,

    free_blocks: VecDeque<BlockId>,
    full_blocks: VecDeque<BlockId>,

    all_blocks: Box<[UnsafeCell<Block>; ALL_BLOCKS]>,
}

#[derive(Debug)]
pub struct Block {
    block_id: Addr,
    valid_counter: Counter,
    cursor: PageId, // points to the next addr
    erase_counter: Counter,
    p2l: Box<[PageId; PAGES_PER_BLOCK]>, // stack overflow could be
    state: BlockState,
}

#[derive(Debug, PartialEq)]
enum BlockState {
    Erased,
    Open,
    Written,
}

impl Block {
    pub fn new(block_id: Addr) -> Self {
        let p2l = Box::new([INVALID_PAGE_ID; PAGES_PER_BLOCK]);
        Block {
            block_id,
            valid_counter: 0,
            cursor: 0,
            erase_counter: 0,
            p2l,
            state: BlockState::Erased,
        }
    }

    fn invalid_counter(&self) -> Counter {
        PAGES_PER_BLOCK - self.valid_counter
    }

    fn full(&self) -> bool {
        self.cursor == PAGES_PER_BLOCK
    }

    fn no_valid_pages(&self) -> bool {
        self.valid_counter == 0
    }

    fn write_one_page(&mut self, lba: PageId) {
        debug_assert!(
            self.valid_counter <= PAGES_PER_BLOCK,
            "write more pages than block has"
        );
        debug_assert!(
            self.p2l[self.cursor] == INVALID_PAGE_ID,
            "lba: {}, blk: {}",
            lba,
            self.block_id
        );

        debug_assert!(
            self.state == BlockState::Open,
            "Writes allowed only to Opened block"
        );

        self.p2l[self.cursor] = lba;

        self.cursor += 1;
        self.valid_counter += 1;

        if self.full() {
            self.state = BlockState::Written;
        }
    }

    fn set_state(&mut self, new_state: BlockState) {
        unimplemented!();
    }

    fn get_physical_for_following_page(&self) -> PageId {
        self.block_id * PAGES_PER_BLOCK + self.cursor
    }

    fn open(&mut self) {
        self.state = BlockState::Open;
    }

    fn erase(&mut self) {
        if self.state != BlockState::Written {
            panic!("Can erase only written block");
        }
        debug_assert!(self.p2l.iter().all(|x| *x == INVALID_PAGE_ID));

        self.cursor = 0;
        debug_assert!(self.valid_counter == 0);
        self.erase_counter += 1;

        self.state = BlockState::Erased;
    }
}

impl Ftl {
    const INIT_HOST_OPEN_BLOCK: BlockId = 0;
    const INIT_GC_OPEN_BLOCK: BlockId = 1;

    pub fn default() -> Self {
        let mut all_blocks: Box<[UnsafeCell<Block>; ALL_BLOCKS]> = Box::new(
            (0..ALL_BLOCKS)
                .map(|x| UnsafeCell::new(Block::new(x)))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        );

        let free_blocks = VecDeque::from((2..ALL_BLOCKS).collect::<Vec<BlockId>>());
        let full_blocks = VecDeque::with_capacity(ALL_BLOCKS);

        all_blocks[Self::INIT_HOST_OPEN_BLOCK].get_mut().state = BlockState::Open;
        all_blocks[Self::INIT_GC_OPEN_BLOCK].get_mut().state = BlockState::Open;

        Ftl {
            // Vec::with_capacity could be faster even in case of wasted memory
            // than empty + reserve
            l2p: Vec::new(),
            logical_size: 0,
            host_open_block_idx: Self::INIT_HOST_OPEN_BLOCK,
            host_gc_open_block_idx: Self::INIT_GC_OPEN_BLOCK,
            free_blocks,
            full_blocks,
            all_blocks,
        }
    }
    pub fn new_with_op(op: f64) -> Self {
        use byte_unit::Byte;

        let mut out = Ftl::default();
        let op_pages = (ALL_PAGES as f64 * (op / 100.)).trunc() as Addr;

        trace!("ALL_PAGES: {}", ALL_PAGES);
        trace!("op_pages: {}", op_pages);
        trace!(
            "Physical Capacity: {} bytes, {}",
            ALL_PAGES * PAGE_SIZE,
            Byte::from(ALL_PAGES * PAGE_SIZE)
                .get_appropriate_unit(true)
                .to_string()
        );
        trace!(
            "User Capacity: {} bytes, {}",
            (ALL_PAGES - op_pages) * PAGE_SIZE,
            Byte::from((ALL_PAGES - op_pages) * PAGE_SIZE)
                .get_appropriate_unit(true)
                .to_string()
        );

        out.logical_size = ALL_PAGES - op_pages;
        out.l2p.reserve(out.logical_size);
        out.l2p.resize(out.logical_size, INVALID_PAGE_ID);

        out
    }

    pub fn get_max_lba(&self) -> Addr {
        self.logical_size - 1
    }

    fn get_next_free_block(&mut self) -> BlockId {
        let blk_id = self.free_blocks.pop_front().unwrap();
        debug_assert!(self.all_blocks[blk_id].get_mut().state == BlockState::Erased);
        blk_id
    }

    fn put_full_block(&mut self, block_id: BlockId) {
        debug_assert!(self.all_blocks[block_id].get_mut().state == BlockState::Written);
        self.full_blocks.push_back(block_id);
    }

    fn put_empty_block(&mut self, block_id: BlockId) {
        debug_assert!(self.all_blocks[block_id].get_mut().state == BlockState::Erased);
        self.free_blocks.push_back(block_id);
    }

    fn get_next_full_block(&mut self) -> BlockId {
        let blk_id = self.full_blocks.pop_front().unwrap();
        debug_assert!(self.all_blocks[blk_id].get_mut().state == BlockState::Written);
        blk_id
    }

    // Open any block before accepting writes
    // And verify that you can open this block for writes before
    fn open_block(&mut self, block_id: BlockId) {
        let blk = self.all_blocks[block_id].get_mut();
        debug_assert!(blk.state == BlockState::Erased);

        blk.state = BlockState::Open;
    }

    fn gc(&mut self) {
        // @TODO: find better victim block, based on valid/invalid pages
        let victim_block_idx = self.get_next_full_block();
        // @DANGEROUS
        // This is dirty hack to workaround with complicated split borrow thing in rust
        // With this pointer dereference we could break rust guarantees of borrow checker
        // but the main purposes of this code is to overcome double mutable borrow from array
        // https://doc.rust-lang.org/nomicon/borrow-splitting.html
        let victim_block = {
            let ptr: *mut Block = self.all_blocks[victim_block_idx].get_mut() as *mut Block;
            unsafe { &mut *ptr }
        };

        if victim_block.no_valid_pages() {
            victim_block.erase();
            self.put_empty_block(victim_block_idx);
            return;
        }

        debug_assert!(victim_block.full()); // in case of random rewrite must be fixed

        // because of unsafe block, we must check this on our own
        assert_ne!(self.host_gc_open_block_idx, victim_block_idx);
        let mut gc_target = self.all_blocks[self.host_gc_open_block_idx].get_mut();

        // copy data from victim to open gc block
        let mut pages_to_substract = 0;
        for lba in victim_block.p2l.iter_mut() {
            if gc_target.full() {
                self.put_full_block(self.host_gc_open_block_idx);
                self.host_gc_open_block_idx = self.get_next_free_block();

                // because of unsafe block, we must check this on our own
                assert_ne!(self.host_gc_open_block_idx, victim_block_idx);
                self.open_block(self.host_gc_open_block_idx);
                gc_target = self.all_blocks[self.host_gc_open_block_idx].get_mut();

                debug_assert!(
                    gc_target.p2l.iter().all(|x| *x == INVALID_PAGE_ID),
                    "Not all phys pages are empty accordind to p2l"
                );
                debug_assert!(
                    self.host_open_block_idx != self.host_gc_open_block_idx,
                    "Can't mix host and GC open blocks"
                );
            }
            if *lba == INVALID_PAGE_ID {
                continue;
            }
            pages_to_substract += 1;

            self.l2p[*lba] = gc_target.get_physical_for_following_page();
            gc_target.write_one_page(*lba);

            *lba = INVALID_PAGE_ID;
        }
        victim_block.valid_counter -= pages_to_substract;
        debug_assert!(victim_block.valid_counter == 0);
        debug_assert!(victim_block.p2l.iter().all(|x| *x == INVALID_PAGE_ID));

        // @TODO: add real page address usage
        victim_block.erase();
        debug_assert!(victim_block.block_id == victim_block_idx);
        debug_assert!(victim_block.state == BlockState::Erased);

        // FIXME: remove clone
        self.put_empty_block(victim_block_idx);
    }

    // @TODO: optimise writes for up to the size of the block to speedup
    pub fn write(&mut self, lba: usize) -> Result<bool, &'static str> {
        debug_assert!(lba < self.logical_size);

        let mut host_open_block = self.all_blocks[self.host_open_block_idx].get_mut();

        if host_open_block.full() {
            debug_assert!(
                host_open_block.state == BlockState::Written,
                "block_id: {}, state: {:?}",
                host_open_block.block_id,
                host_open_block.state
            );
            // host_open_block.state = BlockState::Written;
            // FIXME: remove clone
            self.put_full_block(self.host_open_block_idx);
            self.host_open_block_idx = self.get_next_free_block();

            self.open_block(self.host_open_block_idx);
            host_open_block = self.all_blocks[self.host_open_block_idx].get_mut();

            debug_assert!(
                host_open_block.p2l.iter().all(|x| *x == INVALID_PAGE_ID),
                "Not all phys pages are empty accordind to p2l"
            );
            debug_assert!(
                self.host_open_block_idx != self.host_gc_open_block_idx,
                "Can't mix host and GC open blocks"
            );
        }

        loop {
            if self.free_blocks.len() < FREE_BLOCKS {
                self.gc();
            } else {
                break;
            }
        }

        host_open_block = self.all_blocks[self.host_open_block_idx].get_mut();
        // @TODO: verify host_open_block here
        // debug!("Host OpenBlock addr: {}", self.host_open_block.block_id);
        debug_assert!(
            !host_open_block.full(),
            "host_open_block is full before write"
        );

        // trace!("Write {}", idx);
        if self.l2p[lba] != INVALID_PAGE_ID {
            // FIXME use dedicated function to convert
            let (block_id, page_id) = div_rem(self.l2p[lba], PAGES_PER_BLOCK);
            let dst;

            if block_id == self.host_open_block_idx {
                dst = host_open_block;
            } else if block_id == self.host_gc_open_block_idx {
                dst = self.all_blocks[self.host_gc_open_block_idx].get_mut();
            } else {
                dst = self.all_blocks[block_id].get_mut()
            }

            debug_assert!(dst.state != BlockState::Erased);
            debug_assert!(dst.p2l[page_id] == lba);
            dst.valid_counter -= 1;
            dst.p2l[page_id] = INVALID_PAGE_ID;
        }
        host_open_block = self.all_blocks[self.host_open_block_idx].get_mut();
        // @TODO: verify host_open_block here

        self.l2p[lba] = host_open_block.get_physical_for_following_page();
        host_open_block.write_one_page(lba);

        Ok(true)
    }
}
