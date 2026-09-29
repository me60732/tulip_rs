//! Dump the rolling plans' CLIF for the steady-body census (debug harness
//! for the E3 optimization pass; see Handover E3).
use tulip_rs::cse::{Planable, Request};
use tulip_rs::indicators::bbands::BBands;
use tulip_rs::indicators::hma::Hma;
use tulip_rs::indicators::sma::Sma;
use tulip_rs::indicators::stddev::StdDev;
use tulip_rs::indicators::wma::Wma;

fn main() {
    let mixed = Request::new()
        .add_opt(Sma::recipe(&[5.0]), None)
        .add_opt(Wma::recipe(&[8.0]), Some(&[true]))
        .add_opt(StdDev::recipe(&[14.0]), Some(&[true]))
        .add_opt(Sma::recipe(&[20.0]), None)
        .add_opt(StdDev::recipe(&[20.0]), Some(&[true]))
        .add_opt(BBands::recipe(&[20.0, 2.5]), None)
        .add_opt(Wma::recipe(&[26.0]), None);
    std::fs::write("/tmp/rolling_mixed.clif", mixed.build().clif()).expect("write");
    println!("wrote /tmp/rolling_mixed.clif");

    let hma = Request::new().add_opt(Hma::recipe(&[20.0]), None);
    std::fs::write("/tmp/rolling_hma.clif", hma.build().clif()).expect("write");
    println!("wrote /tmp/rolling_hma.clif");
}
