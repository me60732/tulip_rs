use tulip_rs::indicators::fisher::{Fisher, Indicator, IndicatorByOptions, TIndicatorState};

const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
]; // High prices
const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
]; // Low prices

fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    for _ in 0..10 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
    }
    (high_vec, low_vec)
}

fn main() {
    let (high, low) = expand_inputs();
    let options = [5.0]; // Period

    let inputs = [high.as_slice(), low.as_slice()];

    /////////////////////// Full run ///////////////////////
    let (outputs, _) = match Fisher::indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Fisher Line:        {:?}", outputs[0]);
    println!("Full Fisher Signal Line: {:?}", outputs[1]);

    /////////////////////// Partial run + batch_indicator continuation ///////////////////////
    let inputs2 = [&high[..high.len() - 5], &low[..low.len() - 5]];

    let (outputs2, mut state) = match Fisher::indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial Fisher Line:        {:?}", outputs2[0]);
    println!("Partial Fisher Signal Line: {:?}", outputs2[1]);

    let new_inputs = [&high[high.len() - 5..], &low[low.len() - 5..]];

    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nFinal Fisher Line:        {:?}", final_outputs[0]);
    println!("Final Fisher Signal Line: {:?}", final_outputs[1]);

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    let asset0: [&[f64]; 2] = [&high, &low];
    let asset1: [&[f64]; 2] = [&high, &low];
    let asset2: [&[f64]; 2] = [&high, &low];
    let asset3: [&[f64]; 2] = [&high, &low];
    let inputs_4: [&[&[f64]; 2]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) = match Fisher::indicator_by_assets::<4>(&inputs_4, &options, None)
    {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "\nSIMD by-assets Fisher        (asset 0): {:?}",
        simd_asset_outputs[0][0]
    );
    println!(
        "SIMD by-assets Fisher Signal (asset 0): {:?}",
        simd_asset_outputs[0][1]
    );

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    let options_4 = [&[5.0], &[7.0], &[10.0], &[14.0]];

    let (simd_option_outputs, _) =
        match Fisher::indicator_by_options::<4>(&inputs, &options_4, None) {
            Ok(result) => result,
            Err(e) => panic!("Error: {}", e),
        };
    for (i, opts) in options_4.iter().enumerate() {
        println!(
            "\nSIMD by-options Fisher        (period={}): {:?}",
            opts[0], simd_option_outputs[i][0]
        );
        println!(
            "SIMD by-options Fisher Signal (period={}): {:?}",
            opts[0], simd_option_outputs[i][1]
        );
    }
}
