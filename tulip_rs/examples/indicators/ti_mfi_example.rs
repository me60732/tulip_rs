use tulip_rs::indicators::mfi::{indicator, TIndicatorState};

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
    let volume = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];
    let period = 5.0;
    let options = [period];

    let inputs = [
        high.as_slice(),
        low.as_slice(),
        close.as_slice(),
        volume.as_slice(),
    ];
    // Calculate the MFI using the full dataset
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full MFI Line: {:?}", outputs[0]);
    println!("Full Typical Price Line: {:?}", outputs[1]);

    let inputs2 = [
        &high[..high.len() - 5],
        &low[..low.len() - 5],
        &close[..close.len() - 5],
        &volume[..volume.len() - 5],
    ];

    // Calculate the MFI using the full dataset
    let (outputs2, mut state) = match indicator(&inputs2, &options, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nMFI Line: {:?}", outputs2[0]);
    println!("Typical Price Line: {:?}", outputs2[1]);

    // Use the last 5 inputs for the indicator_from_state function
    let new_inputs = [
        &high[high.len() - 5..],
        &low[low.len() - 5..],
        &close[close.len() - 5..],
        &volume[volume.len() - 5..],
    ];

    // Calculate the MFI using the recent data and previous state
    let new_outputs = match state.batch_indicator(&new_inputs, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nNew MFI Line: {:?}", new_outputs[0]);
    println!("New Typical Price Line: {:?}", new_outputs[1]);
}
