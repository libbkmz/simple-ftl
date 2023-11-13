#![allow(dead_code)]
#![allow(unused)]



mod config;
mod ftl;
#[macro_use] extern crate log;
extern crate simplelog;

#[cfg(not(test))]
use log::{debug, error};

#[cfg(test)]
use std::{println as info, println as warn, println as error, println as trace, println as debug };


use ftl::Ftl;
use simplelog::*;
use time::macros::format_description;
use crate::config::{ALL_PAGES, BLOCKS, PAGES};


fn main() {
    let log_cfg = ConfigBuilder::new()
        .set_time_format_custom(format_description!("[hour]:[minute]:[second].[subsecond]"))
        .build();

    let _ = SimpleLogger::init(LevelFilter::Trace, log_cfg).unwrap();

    let op = 7.0;
    let mut fw = Ftl::new_with_op(op);

    for i in 0..10 {

        fw.write(ALL_PAGES);
        info!("Capacity {} written", i);
    }



}


mod test {
    use crate::config::PAGES;
    use crate::ftl::*;

    #[test]
    fn writes_one_block() {
        let mut fw = Ftl::new_with_op(7.0);
        fw.write(PAGES);
    }

    #[test]
    fn writes_more_than_one_block() {
        let mut fw = Ftl::new_with_op(7.0);
        fw.write(PAGES*2);
    }
}