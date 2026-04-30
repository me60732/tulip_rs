use tulip_rs::indicators::aroonosc::{indicator, TIndicatorState};

fn main() {
    // Example input data: high and low prices
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices

    let inputs = [high.as_slice(), low.as_slice()];

    // Options for the Aroon calculation: period
    let options = [5.0];

    // Calculate the Aroon using the indicator function
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Aroonosc Line: {:?}", outputs[0]);
    println!("Full Aroon Down Line: {:?}", outputs[2]);
    println!("Full Aroon Up Line: {:?}", outputs[1]);

    let inputs2 = [&high[..high.len() - 5], &low[..low.len() - 5]];

    let (outputs2, mut state2) = match indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nAroonosc Line: {:?}", outputs2[0]);

    let new_inputs = [&high[high.len() - 5..], &low[low.len() - 5..]];

    let new_outputs = match state2.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew AroonOsc Line: {:?}", new_outputs[0]);
}
