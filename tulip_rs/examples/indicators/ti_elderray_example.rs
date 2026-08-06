use tulip_rs::indicators::elderray::{
    indicator_by_assets, indicator_by_options, Elderray, Indicator, TIndicatorState,
};

fn main() {
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices (15 bars)
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices (15 bars)
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices (15 bars)
    let options = [5.0]; // EMA period = 5

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

    /////////////////////// Full run — bull and bear power ///////////////////////
    let (outputs, _) = match Elderray::indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Bull Power (high − EMA): {:?}", outputs[0]);
    println!("Bear Power (low  − EMA): {:?}", outputs[1]);

    /////////////////////// Full run with optional EMA output ///////////////////////
    let (outputs_with_ema, _) = match Elderray::indicator(&inputs, &options, Some(&[true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nBull Power (with EMA):   {:?}", outputs_with_ema[0]);
    println!("Bear Power (with EMA):   {:?}", outputs_with_ema[1]);
    println!("EMA line:                {:?}", outputs_with_ema[2]);

    /////////////////////// Partial run + batch_indicator continuation ///////////////////////
    let inputs2 = [
        &high[0..high.len() - 5],
        &low[0..low.len() - 5],
        &close[0..close.len() - 5],
    ];

    let (outputs2, mut state) = match Elderray::indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial Bull Power: {:?}", outputs2[0]);
    println!("Partial Bear Power: {:?}", outputs2[1]);

    let new_inputs = [
        &high[high.len() - 5..],
        &low[low.len() - 5..],
        &close[close.len() - 5..],
    ];

    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nContinued Bull Power: {:?}", final_outputs[0]);
    println!("Continued Bear Power: {:?}", final_outputs[1]);

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    // Each asset has three input series: [high, low, close]
    let asset0: [&[f64]; 3] = [&high, &low, &close];
    let high1: Vec<f64> = high.iter().map(|v| v * 1.2).collect();
    let low1: Vec<f64> = low.iter().map(|v| v * 1.2).collect();
    let close1: Vec<f64> = close.iter().map(|v| v * 1.2).collect();
    let asset1: [&[f64]; 3] = [high1.as_slice(), low1.as_slice(), close1.as_slice()];
    let high2: Vec<f64> = high
        .iter()
        .enumerate()
        .map(|(i, v)| 90.0 + i as f64 * 0.5 + v * 0.1)
        .collect();
    let low2: Vec<f64> = low
        .iter()
        .enumerate()
        .map(|(i, v)| 90.0 + i as f64 * 0.5 + v * 0.1)
        .collect();
    let close2: Vec<f64> = close
        .iter()
        .enumerate()
        .map(|(i, v)| 90.0 + i as f64 * 0.5 + v * 0.1)
        .collect();
    let asset2: [&[f64]; 3] = [high2.as_slice(), low2.as_slice(), close2.as_slice()];
    let high3: Vec<f64> = high
        .iter()
        .enumerate()
        .map(|(i, v)| 100.0 - i as f64 * 0.3 + v * 0.05)
        .collect();
    let low3: Vec<f64> = low
        .iter()
        .enumerate()
        .map(|(i, v)| 100.0 - i as f64 * 0.3 + v * 0.05)
        .collect();
    let close3: Vec<f64> = close
        .iter()
        .enumerate()
        .map(|(i, v)| 100.0 - i as f64 * 0.3 + v * 0.05)
        .collect();
    let asset3: [&[f64]; 3] = [high3.as_slice(), low3.as_slice(), close3.as_slice()];
    let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) = match indicator_by_assets::<4>(&inputs_4, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "\nSIMD by-assets Bull Power (asset 0): {:?}",
        simd_asset_outputs[0][0]
    );
    println!(
        "SIMD by-assets Bear Power (asset 0): {:?}",
        simd_asset_outputs[0][1]
    );

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    // Option sets chosen so all produce output with 15 bars:
    //   period=5  → min_data=6  → 10 output values
    //   period=7  → min_data=8  →  8 output values
    //   period=9  → min_data=10 →  6 output values
    //   period=12 → min_data=13 →  3 output values
    let options_4 = [&[5.0], &[7.0], &[9.0], &[12.0]];

    let (simd_option_outputs, _) = match indicator_by_options::<4>(&inputs, &options_4, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    for (i, opts) in options_4.iter().enumerate() {
        println!(
            "\nSIMD by-options Bull Power (period={}): {:?}",
            opts[0], simd_option_outputs[i][0]
        );
        println!(
            "SIMD by-options Bear Power (period={}): {:?}",
            opts[0], simd_option_outputs[i][1]
        );
    }
}
