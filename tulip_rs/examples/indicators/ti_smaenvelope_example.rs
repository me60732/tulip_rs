use tulip_rs::indicators::smaenvelope::{
    Indicator, IndicatorByOptions, SmaEnvelope, TIndicatorState,
};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices (15 bars)
    let options = [5.0, 2.0]; // Period=5, Percentage=2.0 (envelope width %)

    let inputs = [close.as_slice()];

    /////////////////////// Full run — all three bands ///////////////////////
    let (outputs, _) = match SmaEnvelope::indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Lower Band:  {:?}", outputs[0]);
    println!("Full Middle Band: {:?}", outputs[1]);
    println!("Full Upper Band:  {:?}", outputs[2]);

    /////////////////////// Partial run + batch_indicator continuation ///////////////////////
    let inputs2 = [&close[0..close.len() - 5]];

    let (outputs2, mut state) = match SmaEnvelope::indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial Lower Band:  {:?}", outputs2[0]);
    println!("Partial Middle Band: {:?}", outputs2[1]);
    println!("Partial Upper Band:  {:?}", outputs2[2]);

    let new_inputs = [&close[close.len() - 5..]];

    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nContinued Lower Band:  {:?}", final_outputs[0]);
    println!("Continued Middle Band: {:?}", final_outputs[1]);
    println!("Continued Upper Band:  {:?}", final_outputs[2]);

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    // Each asset has a single 'real' input series
    let asset0: [&[f64]; 1] = [&close];
    let close1: Vec<f64> = close.iter().map(|v| v * 1.2).collect();
    let asset1: [&[f64]; 1] = [close1.as_slice()];
    let close2: Vec<f64> = close
        .iter()
        .enumerate()
        .map(|(i, v)| 90.0 + i as f64 * 0.5 + v * 0.1)
        .collect();
    let asset2: [&[f64]; 1] = [close2.as_slice()];
    let close3: Vec<f64> = close
        .iter()
        .enumerate()
        .map(|(i, v)| 100.0 - i as f64 * 0.3 + v * 0.05)
        .collect();
    let asset3: [&[f64]; 1] = [close3.as_slice()];
    let inputs_4: [&[&[f64]; 1]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) =
        match SmaEnvelope::indicator_by_assets::<4>(&inputs_4, &options, None) {
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
    // All option sets produce output with 15 bars:
    //   period=5  → min_data=6  → 10 output values
    //   period=7  → min_data=8  → 8 output values
    //   period=10 → min_data=11 → 5 output values
    //   period=12 → min_data=13 → 3 output values
    let options_4 = [&[5.0, 2.0], &[7.0, 3.0], &[10.0, 2.0], &[12.0, 5.0]];

    let (simd_option_outputs, _) =
        match SmaEnvelope::indicator_by_options::<4>(&inputs, &options_4, None) {
            Ok(result) => result,
            Err(e) => panic!("Error: {}", e),
        };
    for (i, opts) in options_4.iter().enumerate() {
        println!(
            "\nSIMD by-options Lower  (period={}, pct={}%): {:?}",
            opts[0], opts[1], simd_option_outputs[i][0]
        );
        println!(
            "SIMD by-options Middle (period={}, pct={}%): {:?}",
            opts[0], opts[1], simd_option_outputs[i][1]
        );
        println!(
            "SIMD by-options Upper  (period={}, pct={}%): {:?}",
            opts[0], opts[1], simd_option_outputs[i][2]
        );
    }
}
