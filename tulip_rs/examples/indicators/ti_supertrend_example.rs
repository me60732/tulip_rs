use tulip_rs::indicators::supertrend::{
    indicator_by_assets, indicator_by_options, Indicator, SuperTrend, TIndicatorState,
};

fn main() {
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
    // period = 5 (ATR smoothing window), step = 3.0 (ATR multiplier for bands)
    let options = [5.0, 3.0];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

    /////////////////////// Full run with all optional outputs ///////////////////////
    // optional_outputs: [want_atr, want_tr, want_medprice]
    let (outputs, _) = match SuperTrend::indicator(&inputs, &options, Some(&[true, true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full SuperTrend Line: {:?}", outputs[0]);
    println!("Full ATR Line:        {:?}", outputs[1]);
    println!("Full TR Line:         {:?}", outputs[2]);
    println!("Full MedPrice Line:   {:?}", outputs[3]);

    /////////////////////// Full run with no optional outputs ///////////////////////
    let (outputs_plain, _) = match SuperTrend::indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "\nFull SuperTrend Line (no optional): {:?}",
        outputs_plain[0]
    );

    /////////////////////// Partial run + batch_indicator continuation ///////////////////////
    let n = high.len() - 5;
    let inputs2 = [&high[..n], &low[..n], &close[..n]];

    let (outputs2, mut state) = match SuperTrend::indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial SuperTrend Line: {:?}", outputs2[0]);

    // Continue from saved state using the remaining 5 bars
    let new_inputs = [&high[n..], &low[n..], &close[n..]];
    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Continued SuperTrend Line: {:?}", final_outputs[0]);

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    // Each asset shares the same period and step; SIMD processes all 4 in parallel.
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
        "\nSIMD by-assets SuperTrend (asset 0): {:?}",
        simd_asset_outputs[0][0]
    );
    println!(
        "SIMD by-assets SuperTrend (asset 1): {:?}",
        simd_asset_outputs[1][0]
    );

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    // Each lane uses a different [period, step] pair; SIMD processes all 4 simultaneously.
    let options_4 = [
        &[5.0_f64, 3.0],
        &[7.0_f64, 2.5],
        &[10.0_f64, 3.0],
        &[14.0_f64, 2.0],
    ];

    let (simd_option_outputs, _) = match indicator_by_options::<4>(&inputs, &options_4, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    for (i, opts) in options_4.iter().enumerate() {
        println!(
            "\nSIMD by-options SuperTrend (period={}, step={}): {:?}",
            opts[0], opts[1], simd_option_outputs[i][0]
        );
    }
}
