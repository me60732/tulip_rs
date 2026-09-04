use tulip_rs::indicators::ef::{Ef, Indicator, IndicatorByOptions, TIndicatorState};

fn main() {
    let real = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Price series
    let options = [5.0]; // Period

    let inputs = [real.as_slice()];

    /////////////////////// Full run ///////////////////////
    let (outputs, _) = match Ef::indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full EF Line: {:?}", outputs[0]);

    /////////////////////// Partial run + batch_indicator continuation ///////////////////////
    let inputs2 = [&real[0..real.len() - 5]];

    let (outputs2, mut state) = match Ef::indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial EF Line: {:?}", outputs2[0]);

    let new_inputs = [&real[real.len() - 5..]];

    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nFinal EF Line: {:?}", final_outputs[0]);

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    let asset0: [&[f64]; 1] = [&real];
    let asset1: [&[f64]; 1] = [&real];
    let asset2: [&[f64]; 1] = [&real];
    let asset3: [&[f64]; 1] = [&real];
    let inputs_4: [&[&[f64]; 1]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) = match Ef::indicator_by_assets::<4>(&inputs_4, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "\nSIMD by-assets EF (asset 0): {:?}",
        simd_asset_outputs[0][0]
    );

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    let options_4 = [&[3.0], &[5.0], &[10.0], &[14.0]];

    let (simd_option_outputs, _) = match Ef::indicator_by_options::<4>(&inputs, &options_4, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    for (i, opts) in options_4.iter().enumerate() {
        println!(
            "\nSIMD by-options EF (period={}): {:?}",
            opts[0], simd_option_outputs[i][0]
        );
    }
}
