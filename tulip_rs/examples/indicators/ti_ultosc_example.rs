use tulip_rs::indicators::ultosc::{indicator, TIndicatorState};

fn main() {
    // Sample OHLC data (from your test file)
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
    ];

    // Options: short_period, medium_period, long_period.
    // They must follow: short_period <= medium_period <= long_period. Example: 3, 5, 7.
    let options = [2.0, 3.0, 5.0];

    // Arrange inputs in expected order.
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

    // Call the ultosc indicator function.
    let (result, _) = match indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {:?}", e),
    };
    println!("Full Ultosc Line: {:?}", result[0]);

    let inputs2 = [
        &high[..high.len() - 5],
        &low[..low.len() - 5],
        &close[..close.len() - 5],
    ];
    let (result2, mut state2) = match indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {:?}", e),
    };
    println!("\nPartial Ultosc Line: {:?}", result2[0]);

    let inputs3 = [
        &high[high.len() - 5..],
        &low[low.len() - 5..],
        &close[close.len() - 5..],
    ];

    let result = match state2.batch_indicator(&inputs3, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {:?}", e),
    };

    println!("\nFinal Ultosc Line: {:?}", result[0]);
}
