use tulip_rs::indicators::natr::{indicator, TIndicatorState};
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
    let inputs = [
        &high[..high.len() - 1],
        &low[..low.len() - 1],
        &close[..close.len() - 1],
    ];
    let options = [5.0]; // Period

    // Calculate the Average True Range (ATR) line using the indicator function
    let (outputs, mut state) = match indicator(&inputs, &options, None) {
        //Some(&[true, true])){
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("NATR Line: {:?}", outputs[0]);
    //println!("ATR Line: {:?}", atr_line);
    //println!("TR Line: {:?}", tr_line);

    let new_data = [
        &high[high.len() - 1..],
        &low[low.len() - 1..],
        &close[close.len() - 1..],
    ];

    // Calculate the new ATR line using the previous close value as the starting point
    let new_outputs = match state.batch_indicator(&new_data, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew ATR Line: {:?}", new_outputs[0]);
}
