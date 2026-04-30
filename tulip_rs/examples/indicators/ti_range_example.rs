//use tulip_rs::common::IndicatorState;
use tulip_rs::indicators::range::indicator;

fn main() {
    let high = [
        82.15, 81.89, 83.03, 83.3, 83.85, 83.9, 83.33, 84.3, 84.84, 85.0, 85.9, 86.58, 86.98, 88.0,
        87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.3, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices

    let inputs = [high.as_slice(), low.as_slice()];
    let options = [];
    let results = match indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => {
            println!("Error: {:?}", e.message());
            return;
        }
    };
    println!("Range: {:?}", results.indicators[0]);
}
