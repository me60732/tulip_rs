use tulip_rs::indicators::chaikinmf::{ChaikinMf, Indicator, TIndicatorState};

fn main() {
    // Test Input Data
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87, 88.50, 89.20, 89.75, 90.10, 89.80f64,
    ];
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01, 87.60, 88.15, 88.90, 89.40, 88.95f64,
    ];
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29, 88.10, 88.80, 89.50, 89.95, 89.25f64,
    ];
    let volume = [
        5653100.0,
        6447400.0,
        7690900.0,
        3831400.0,
        4455100.0,
        3798000.0,
        3936200.0,
        4732000.0,
        4841300.0,
        3915300.0,
        6830800.0,
        6694100.0,
        5293600.0,
        7985800.0,
        4807900.0,
        5100000.0,
        6200000.0,
        5800000.0,
        7100000.0,
        4900000.0f64,
    ];
    let period = 5.0;
    let options = [period];

    let inputs = [
        high.as_slice(),
        low.as_slice(),
        close.as_slice(),
        volume.as_slice(),
    ];

    // Calculate the Chaikin Money Flow using the full dataset
    let (outputs, _) = match ChaikinMf::indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full CMF Line: {:?}", outputs[0]);

    let n = high.len() - 5;
    let inputs2 = [&high[..n], &low[..n], &close[..n], &volume[..n]];

    // Calculate the Chaikin Money Flow using a partial dataset
    let (outputs2, mut state) = match ChaikinMf::indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial CMF Line: {:?}", outputs2[0]);

    // Use the last 5 inputs to continue from the saved state
    let new_inputs = [&high[n..], &low[n..], &close[n..], &volume[n..]];

    // Calculate the Chaikin Money Flow using the recent data and previous state
    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nContinued CMF Line: {:?}", new_outputs[0]);
}
