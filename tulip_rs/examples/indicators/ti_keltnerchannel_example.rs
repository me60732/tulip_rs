use tulip_rs::indicators::keltnerchannel::{
    indicator, indicator_by_assets, indicator_by_options, TIndicatorState,
};

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
    let options = [5.0, 2.0]; // Period, Step multiplier

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

    /////////////////////// Full run with optional ATR and TR outputs ///////////////////////
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Lower Band: {:?}", outputs[0]);
    println!("Full Middle Band (EMA): {:?}", outputs[1]);
    println!("Full Upper Band: {:?}", outputs[2]);
    println!("Full ATR Line:   {:?}", outputs[3]);
    println!("Full TR Line:    {:?}", outputs[4]);

    /////////////////////// Partial run + batch_indicator continuation ///////////////////////
    let inputs2 = [
        &high[0..high.len() - 5],
        &low[0..low.len() - 5],
        &close[0..close.len() - 5],
    ];

    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial Lower Band:  {:?}", outputs2[0]);
    println!("Partial Middle Band: {:?}", outputs2[1]);
    println!("Partial Upper Band:  {:?}", outputs2[2]);

    let new_inputs = [
        &high[high.len() - 5..],
        &low[low.len() - 5..],
        &close[close.len() - 5..],
    ];

    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nFinal Lower Band:  {:?}", final_outputs[0]);
    println!("Final Middle Band: {:?}", final_outputs[1]);
    println!("Final Upper Band:  {:?}", final_outputs[2]);

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    let asset0: [&[f64]; 3] = [&high, &low, &close];
    let asset1: [&[f64]; 3] = [&high, &low, &close];
    let asset2: [&[f64]; 3] = [&high, &low, &close];
    let asset3: [&[f64]; 3] = [&high, &low, &close];
    let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

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
    let options_4 = [&[5.0, 2.0], &[7.0, 1.5], &[10.0, 2.0], &[14.0, 2.0]];

    let (simd_option_outputs, _) = match indicator_by_options::<4>(&inputs, &options_4, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    for (i, opts) in options_4.iter().enumerate() {
        println!(
            "\nSIMD by-options Lower  (period={}, step={}): {:?}",
            opts[0], opts[1], simd_option_outputs[i][0]
        );
        println!(
            "SIMD by-options Middle (period={}, step={}): {:?}",
            opts[0], opts[1], simd_option_outputs[i][1]
        );
        println!(
            "SIMD by-options Upper  (period={}, step={}): {:?}",
            opts[0], opts[1], simd_option_outputs[i][2]
        );
    }
}
