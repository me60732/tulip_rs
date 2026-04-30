use tulip_rs::indicators::cci::{indicator, TIndicatorState};

fn main() {
    // Example input data: high, low, and close prices
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

    // Options for the CCI calculation: period
    let options = [5.0];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

    let (indicators, _) = match indicator(&inputs, &options, Some(&[true, true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Full CCI Line: {:?}", indicators[0]);
    println!("\nFull SMA Line: {:?}", indicators[1]);
    println!("\nFull MD Line: {:?}", indicators[2]);
    println!("\nFull TYPPRICE Line: {:?}", indicators[3]);

    let inputs2 = [
        &high[0..high.len() - 5],
        &low[0..low.len() - 5],
        &close[0..close.len() - 5],
    ];

    // Calculate the CCI using the indicator function
    let (indicators, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nCCI Line: {:?}", indicators[0]);

    let new_inputs = [
        &high[high.len() - 5..],
        &low[low.len() - 5..],
        &close[close.len() - 5..],
    ];

    let indicators = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew CCI Line: {:?}", indicators[0]);
}
