use tulip_rs::indicators::donchianchannel::{
    indicator, indicator_by_assets, indicator_by_options, TIndicatorState,
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
    let options = [5.0]; // Period=5

    let inputs = [high.as_slice(), low.as_slice()];

    /////////////////////// Full run — lower, middle, upper bands ///////////////////////
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Lower Band  (lowest low):   {:?}", outputs[0]);
    println!("Full Middle Band ((upper+lower)/2): {:?}", outputs[1]);
    println!("Full Upper Band  (highest high): {:?}", outputs[2]);

    /////////////////////// Partial run + batch_indicator continuation ///////////////////////
    let inputs2 = [&high[0..high.len() - 5], &low[0..low.len() - 5]];

    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial Lower Band:  {:?}", outputs2[0]);
    println!("Partial Middle Band: {:?}", outputs2[1]);
    println!("Partial Upper Band:  {:?}", outputs2[2]);

    let new_inputs = [&high[high.len() - 5..], &low[low.len() - 5..]];

    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nContinued Lower Band:  {:?}", final_outputs[0]);
    println!("Continued Middle Band: {:?}", final_outputs[1]);
    println!("Continued Upper Band:  {:?}", final_outputs[2]);

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    // Each asset has two input series: [high, low]
    let asset0: [&[f64]; 2] = [&high, &low];
    let high1: Vec<f64> = high.iter().map(|v| v * 1.2).collect();
    let low1: Vec<f64> = low.iter().map(|v| v * 1.2).collect();
    let asset1: [&[f64]; 2] = [high1.as_slice(), low1.as_slice()];
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
    let asset2: [&[f64]; 2] = [high2.as_slice(), low2.as_slice()];
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
    let asset3: [&[f64]; 2] = [high3.as_slice(), low3.as_slice()];
    let inputs_4: [&[&[f64]; 2]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) = match indicator_by_assets::<4>(&inputs_4, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "\nSIMD by-assets Lower  (asset 0): {:?}",
        simd_asset_outputs[0][0]
    );
    println!(
        "SIMD by-assets Middle (asset 0): {:?}",
        simd_asset_outputs[0][1]
    );
    println!(
        "SIMD by-assets Upper  (asset 0): {:?}",
        simd_asset_outputs[0][2]
    );

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    // Option sets chosen so all produce output with 15 bars:
    //   period=5  → min_data=6  → 10 output values
    //   period=7  → min_data=8  → 8 output values
    //   period=9  → min_data=10 → 6 output values
    //   period=12 → min_data=13 → 3 output values
    // (period=14 would give only 1 output with 15 bars — expand data for larger periods)
    let options_4 = [&[5.0], &[7.0], &[9.0], &[12.0]];

    let (simd_option_outputs, _) = match indicator_by_options::<4>(&inputs, &options_4, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    for (i, opts) in options_4.iter().enumerate() {
        println!(
            "\nSIMD by-options Lower  (period={}): {:?}",
            opts[0], simd_option_outputs[i][0]
        );
        println!(
            "SIMD by-options Middle (period={}): {:?}",
            opts[0], simd_option_outputs[i][1]
        );
        println!(
            "SIMD by-options Upper  (period={}): {:?}",
            opts[0], simd_option_outputs[i][2]
        );
    }
}
