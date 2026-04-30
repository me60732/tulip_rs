use tulip_rs::indicators::adx::{indicator, TIndicatorState};

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

    let inputs = [&high[..], &low[..], &close[..]];

    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full adx line: {:?}", outputs[0]);
    println!("Full dx line: {:?}", outputs[1]);
    println!("Full atr line: {:?}", outputs[2]);
    println!("Full tr line: {:?}", outputs[3]);


    let inputs2 = [
        &high[0..high.len() - 5], 
        &low[0..low.len() - 5], 
        &close[0..close.len() - 5]
    ];

    // Calculate the ADX using the adx indicator function
    let (outputs2, mut state2) = match indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nPartial ADX Line: {:?}", outputs2[0]);

    let new_high_vec = &high[high.len() - 5..];
    let new_low_vec = &low[low.len() - 5..];
    let new_close_vec = &close[close.len() - 5..];
    let new_inputs = [new_high_vec, new_low_vec, new_close_vec];

    // Calculate the new ADX lines using the previous ADX values as the starting point
    let final_outputs = match state2.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nFinal ADX Line: {:?}", final_outputs[0]);
}
