use tulip_rs::indicators::emv::{indicator, TIndicatorState};

fn main() {
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];
    let volume = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];

    let inputs = [
        &high[0..high.len() - 5],
        &low[0..low.len() - 5],
        &volume[0..volume.len() - 5],
    ];
    let options = [];

    // Example with recent_only parameter set to false
    let (outputs, mut state) = match indicator(&inputs, &options, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("EMV{:?}", outputs[0]);
    println!("MEDPRICE{:?}", outputs[1]);

    let new_inputs = [
        &high[high.len() - 5..],
        &low[low.len() - 5..],
        &volume[volume.len() - 5..],
    ];

    // Example with previously calculated EMV values
    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew EMV Line: {:?}", new_outputs[0]);
}
