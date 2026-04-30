use tulip_rs::indicators::adxr::{indicator, TIndicatorState};

fn main() {
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let options = [5.0]; // Period

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

    let (indicators, _) = match indicator(&inputs, &options, Some(&[true, true, true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full ADXR Line: {:?}", indicators[0]);
    println!("Full ADX Line: {:?}", indicators[1]);
    println!("Full DX Line: {:?}", indicators[2]);
    println!("Full ATR Line: {:?}", indicators[3]);
    println!("Full TR Line: {:?}", indicators[4]);

    let inputs2 = [
        &high[0..high.len() - 2],
        &low[0..low.len() - 2],
        &close[0..close.len() - 2],
    ];

    let (indicators, mut state) =
        match indicator(&inputs2, &options, Some(&[true, true, true, true])) {
            Ok(r) => r,
            Err(e) => panic!("Error: {}", e),
        };

    println!("\n\nPartial ADXR Line: {:?}", indicators[0]);
    println!("Partial ADX Line: {:?}", indicators[1]);
    println!("Partial DX Line: {:?}", indicators[2]);
    println!("Partial ATR Line: {:?}", indicators[3]);
    println!("Partial TR Line: {:?}", indicators[4]);

    let new_inputs = [
        &high[high.len() - 2..],
        &low[low.len() - 2..],
        &close[close.len() - 2..],
    ];

    let indicators = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\n\nFinal ADXR Line: {:?}", indicators[0]);
    println!("Final ADX Line: {:?}", indicators[1]);
    println!("Final DX Line: {:?}", indicators[2]);
    println!("Final ATR Line: {:?}", indicators[3]);
    println!("Final TR Line: {:?}", indicators[4]);
}
