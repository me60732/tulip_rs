//use tulip_rs::common::IndicatorState;
use tulip_rs::indicators::pivotpoint::indicator;
fn main() {
    let high = [
        82.15, 81.89, 83.03, 83.3, 83.85, 83.9, 83.33, 84.3, 84.84, 85.0, 85.9, 86.58, 86.98, 88.0,
        87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.3, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices
    let close = [
        81.59, 81.06, 82.87, 83.0, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let options = [5.0];
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => {
            println!("Error: {:?}", e.message());
            return;
        }
    };
    println!(
        "Pivot Points: s3: {:?}, s2: {:?}, s1: {:?}, pp: {:?}, r1: {:?}, r2: {:?}, r3: {:?}",
        outputs[0][0],
        outputs[0][1],
        outputs[0][2],
        outputs[0][3],
        outputs[0][4],
        outputs[0][5],
        outputs[0][6]
    );
}
