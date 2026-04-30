use tulip_rs::indicators::mass::{indicator, TIndicatorState};

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
    let options = [5.0];

    let mut new_high = high.to_vec();
    new_high.extend(high);
    let mut new_low = low.to_vec();
    new_low.extend(low);
    let inputs = [new_high.as_slice(), new_low.as_slice()];

    // Calculate the Mass Index using the full dataset
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Mass Line: {:?}", outputs[0]);

    let inputs2 = [
        &new_high[..new_high.len() - 5],
        &new_low[..new_low.len() - 5],
    ];

    // Calculate the Mass Index using a partial dataset
    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial Mass Line: {:?}", outputs2[0]);

    let new_inputs = [
        &new_high[new_high.len() - 5..],
        &new_low[new_low.len() - 5..],
    ];

    // Calculate the Mass Index using the recent data and previous state
    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nFinal Mass Line: {:?}", new_outputs[0]);
}
