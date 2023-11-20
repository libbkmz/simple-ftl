#![allow(dead_code)]
#![allow(unused)]
#![allow(non_snake_case)]

mod config;
mod ftl;

#[macro_use]
extern crate log;
extern crate simplelog;

#[cfg(not(test))]
use log::{debug, error};

#[cfg(test)]
use std::{println as info, println as warn, println as error, println as trace, println as debug};

use crate::config::*;
use ftl::Ftl;
use rand::prelude::*;
use simplelog::*;
use time::macros::format_description;

fn main() {
    let log_cfg = ConfigBuilder::new()
        .set_time_format_custom(format_description!("[hour]:[minute]:[second].[subsecond]"))
        .build();

    SimpleLogger::init(LevelFilter::Trace, log_cfg).unwrap();

    let op = 7.0;
    let mut fw = Ftl::new_with_op(op);
    let max_lba = fw.get_max_lba();

    for i in 0..=max_lba {
        fw.write(i as PageId).unwrap();
    }
    info!("Drive preconditioned");

    let mut rng: SmallRng = SmallRng::seed_from_u64(7);

    for c in 0..4 {
        for _ in 0..=max_lba {
            let lba = rng.gen_range(0..=max_lba);
            fw.write(lba).unwrap();
        }
        info!("Capacity {} randomly written", c);
    }
}

mod test {
    use crate::config::PAGES_PER_BLOCK;
    use crate::ftl::*;

    #[test]
    fn writes_one_block() {
        let mut fw = Ftl::new_with_op(7.0);
        fw.write(PAGES_PER_BLOCK);
    }

    #[test]
    fn writes_more_than_one_block() {
        let mut fw = Ftl::new_with_op(7.0);
        fw.write(PAGES_PER_BLOCK * 2);
    }

    #[test]
    fn writes_two_capacities_seq() {
        let mut fw = Ftl::new_with_op(7.0);

        fw.write(fw.get_max_lba() - 1);
        fw.write(fw.get_max_lba() - 1);
    }
}
