use tulip_rs::indicators::hma::{indicator, TIndicatorState};
use tulip_rs::indicators::simd_indicators::hma_simd::indicator_by_assets;

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let options = [5.0];

    // Basic single asset example
    println!("Basic HMA Example");
    println!("=================");

    let inputs = [close.as_slice()];

    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Full HMA Line: {:?}", outputs[0]);

    let inputs2 = [&close[0..close.len() - 1]];
    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial HMA Line: {:?}", outputs2[0]);

    let new_inputs = [&close[close.len() - 1..]];

    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew HMA Line: {:?}", new_outputs[0]);

    // Create variable length data for multiple stocks using extend
    // Stock 1: base length
    let stock1_close = close.to_vec();

    // Stock 2: base length * 2
    let mut stock2_close = close.to_vec();
    stock2_close.extend(&close);

    // Stock 3: base length * 3
    let mut stock3_close = close.to_vec();
    stock3_close.extend(&close);
    stock3_close.extend(&close);

    // Stock 4: base length * 4
    let mut stock4_close = close.to_vec();
    stock4_close.extend(&close);
    stock4_close.extend(&close);
    stock4_close.extend(&close);

    let stocks = vec![
        ("STOCK1", &stock1_close[..]),
        ("STOCK2", &stock2_close[..]),
        ("STOCK3", &stock3_close[..]),
        ("STOCK4", &stock4_close[..]),
    ];

    // Prepare inputs for SIMD processing - we need 4 assets for SIMD width of 4
    let simd_inputs = [
        &[stock1_close.as_slice()], // Stock 1 inputs
        &[stock2_close.as_slice()], // Stock 2 inputs
        &[stock3_close.as_slice()], // Stock 3 inputs
        &[stock4_close.as_slice()], // Stock 4 inputs
    ];

    match indicator_by_assets::<4>(&simd_inputs, &options, None) {
        Ok((simd_outputs, _simd_states)) => {
            println!("\nSIMD processing successful!");
            for (i, (symbol, _)) in stocks.iter().enumerate() {
                println!(
                    "\n\n{}: SIMD HMA ({} values): {:?}",
                    symbol,
                    simd_outputs[i][0].len(),
                    simd_outputs[i][0]
                );
            }
        }
        Err(e) => {
            println!("SIMD processing failed: {}", e);
            println!("Note: SIMD processing requires nightly Rust features");
        }
    }
}
