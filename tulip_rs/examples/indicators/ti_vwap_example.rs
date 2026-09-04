use tulip_rs::indicators::vwap::{Vwap, TIndicatorState, Indicator};

fn main() {
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87f64,
    ];
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01f64,
    ];
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29f64,
    ];
    let volume = [
        5_653_100.0,
        6_447_400.0,
        7_690_900.0,
        3_831_400.0,
        4_455_100.0,
        3_798_000.0,
        3_936_200.0,
        4_732_000.0,
        4_841_300.0,
        3_915_300.0,
        6_830_800.0,
        6_694_100.0,
        5_293_600.0,
        7_985_800.0,
        4_807_900.0f64,
    ];

    // VWAP takes no options; pass an empty array.
    let inputs = [
        high.as_slice(),
        low.as_slice(),
        close.as_slice(),
        volume.as_slice(),
    ];

    /////////////////////// Full run with optional TypPrice output ///////////////////////
    // optional_outputs: [want_typprice]
    let (outputs, _) = match Vwap::indicator(&inputs, &[], Some(&[true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full VWAP Line:          {:?}", outputs[0]);
    println!("Full TypPrice Line:      {:?}", outputs[1]);

    /////////////////////// Full run with no optional outputs ///////////////////////
    let (outputs_plain, _) = match Vwap::indicator(&inputs, &[], None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nFull VWAP Line (plain): {:?}", outputs_plain[0]);

    /////////////////////// Partial run + batch_indicator continuation ///////////////////////
    // VWAP is cumulative from the first bar, so the state carries running
    // pv_sum and vol_sum across calls.
    let n = high.len() - 5;
    let inputs2 = [&high[..n], &low[..n], &close[..n], &volume[..n]];

    let (outputs2, mut state) = match Vwap::indicator(&inputs2, &[], None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial VWAP Line: {:?}", outputs2[0]);

    // Continue from saved state using the remaining 5 bars
    let new_inputs = [&high[n..], &low[n..], &close[n..], &volume[n..]];
    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Continued VWAP Line: {:?}", final_outputs[0]);

    /////////////////////// SIMD by-assets: 4 assets, no options ///////////////////////
    // VWAP has no configurable options; SIMD-by-assets is the primary SIMD entry point.
    let asset0: [&[f64]; 4] = [&high, &low, &close, &volume];
    let asset1: [&[f64]; 4] = [&high, &low, &close, &volume];
    let asset2: [&[f64]; 4] = [&high, &low, &close, &volume];
    let asset3: [&[f64]; 4] = [&high, &low, &close, &volume];
    let inputs_4: [&[&[f64]; 4]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) = match Vwap::indicator_by_assets::<4>(&inputs_4, &[], None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "\nSIMD by-assets VWAP (asset 0): {:?}",
        simd_asset_outputs[0][0]
    );
    println!(
        "SIMD by-assets VWAP (asset 1): {:?}",
        simd_asset_outputs[1][0]
    );
}
