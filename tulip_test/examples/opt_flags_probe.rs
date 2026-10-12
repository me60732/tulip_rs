//! Does the registry respect optional-output flags? Primary-only must HIT;
//! Verify optional-output flags work at runtime (not signature-level).
//! With the new runtime model, enabling optionals doesn't change Signature;
//! it only affects which rows are populated vs empty.
use tulip_rs::cse::{run_registered, Planable, Request};
use tulip_rs::indicators::natr::Natr;
use tulip_rs::indicators::supertrend::SuperTrend;
use tulip_test::database::{get_all_stock_data, init_database_data, EodData};

fn cols(sym: usize, f: impl Fn(&EodData) -> f64) -> Vec<f64> {
    init_database_data();
    get_all_stock_data().unwrap()[sym]
        .1
        .iter()
        .map(|d| f(d))
        .collect()
}

fn main() {
    let hi = cols(0, |d| d.high);
    let lo = cols(0, |d| d.low);
    let cl = cols(0, |d| d.close);
    let three: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), cl.as_slice()];

    let primary = Request::new()
        .add(Natr::recipe(&[14.0]), None)
        .add(SuperTrend::recipe(&[14.0, 3.0]), None);
    let out = run_registered(&primary, &three).expect("primary-only must hit");
    println!(
        "primary-only:      HIT, {} rows (natr at 0, st at 3, optionals disabled)",
        out.rows.len()
    );
    assert_eq!(out.rows[0].len(), 6705 - 14, "natr row length");
    assert_eq!(out.rows[3].len(), 6705 - 14, "st row length");

    // natr optionals = [atr, tr]; supertrend optionals = [atr, tr, medprice]
    let _one_on = Request::new()
        .add(Natr::recipe(&[14.0]), Some(&[true, false]))
        .add(SuperTrend::recipe(&[14.0, 3.0]), None);
    println!(
        "one optional on:   {}",
        if one_on_is_miss(&_one_on, &three) {
            "MISS — BUG"
        } else {
            "HIT — correct (flags only affect row content)"
        }
    );

    let _all_on = Request::new()
        .add(Natr::recipe(&[14.0]), Some(&[true, true]))
        .add(SuperTrend::recipe(&[14.0, 3.0]), Some(&[true, true, true]));
    println!(
        "every optional on: {}",
        if run_registered(&_all_on, &three).is_none() {
            "MISS — BUG"
        } else {
            "HIT — correct (flags only affect row content)"
        }
    );
}

fn one_on_is_miss(r: &Request, s: &[&[f64]; 3]) -> bool {
    run_registered(r, s).is_none()
}
