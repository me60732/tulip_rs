use tulip_rs::indicators::stoch::{indicator, TIndicatorState};

fn main() {
    // Test Input Data
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

    // Options
    let options = [5.0, 3.0, 3.0]; // Period

    // Calculate the Stochastic Oscillator
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(output) => output,
        Err(e) => panic!("Error calculating Stochastic Oscillator: {:?}", e),
    };
    println!("Full Stochastic Oscillator %K Line: {:?}", outputs[0]);
    println!("Full Stochastic Oscillator %D Line: {:?}", outputs[1]);

    let inputs2 = [
        &high[..high.len() - 1],
        &low[..low.len() - 1],
        &close[..close.len() - 1],
    ];

    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(output) => output,
        Err(e) => panic!("Error calculating Stochastic Oscillator: {:?}", e),
    };
    println!("\nStochastic Oscillator %K Line: {:?}", outputs2[0]);
    println!("Stochastic Oscillator %D Line: {:?}", outputs2[1]);

    let new_inputs = [
        &high[high.len() - 1..],
        &low[low.len() - 1..],
        &close[close.len() - 1..],
    ];

    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(output) => output,
        Err(e) => panic!("Error calculating Stochastic Oscillator: {:?}", e),
    };
    println!("\nNew Stochastic Oscillator %K Line: {:?}", new_outputs[0]);
    println!("New Stochastic Oscillator %D Line: {:?}", new_outputs[1]);
}
