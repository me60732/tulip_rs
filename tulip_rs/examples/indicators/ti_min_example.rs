use tulip_rs::indicators::min::{Indicator, IndicatorByOptions, Min, TIndicatorState};

const CLOSE: [f64; 20] = [
    4.3879, 4.3324, 4.3805, 4.4249, 4.2768, 4.2879, 4.5545, 4.6656, 4.5271, 4.3805, 4.4435, 4.4657,
    4.4472, 4.4879, 4.4064, 4.3879, 4.3361, 4.3064, 4.3253, 4.3016,
];

fn expand_inputs() -> Vec<f64> {
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..10 {
        close_vec.extend_from_slice(&CLOSE);
    }
    close_vec
}

fn main() {
    let close = expand_inputs();
    let options = [5.0]; // Period

    let inputs = [close.as_slice()];

    /////////////////////// Full run ///////////////////////
    let (outputs, _) = match Min::indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Min Line: {:?}", outputs[0]);

    /////////////////////// Partial run + batch_indicator continuation ///////////////////////
    let inputs2 = [&close[..close.len() - 5]];

    let (outputs2, mut state) = match Min::indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial Min Line: {:?}", outputs2[0]);

    let new_inputs = [&close[close.len() - 5..]];

    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nFinal Min Line: {:?}", final_outputs[0]);

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    let asset0: [&[f64]; 1] = [&close];
    let asset1: [&[f64]; 1] = [&close];
    let asset2: [&[f64]; 1] = [&close];
    let asset3: [&[f64]; 1] = [&close];
    let inputs_4: [&[&[f64]; 1]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) = match Min::indicator_by_assets::<4>(&inputs_4, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "\nSIMD by-assets Min (asset 0): {:?}",
        simd_asset_outputs[0][0]
    );

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    let options_4 = [&[5.0], &[8.0], &[10.0], &[14.0]];

    let (simd_option_outputs, _) = match Min::indicator_by_options::<4>(&inputs, &options_4, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    for (i, opts) in options_4.iter().enumerate() {
        println!(
            "\nSIMD by-options Min (period={}): {:?}",
            opts[0], simd_option_outputs[i][0]
        );
    }
}
