use tulip_rs::indicators::candlestick::{indicator, ForcastType};

fn main() {
    // Example input data: open, high, low, and close prices
    let mut open = vec![
        81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18,
        88.00, 87.30,
    ];
    let mut high = vec![
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.31,
    ];
    let mut low = vec![
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.20,
    ];
    let mut close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    let pattern_open = vec![87.30, 87.50];
    let pattern_high = vec![88.50, 88.10];
    let pattern_low = vec![87.20, 87.35];
    let pattern_close = vec![88.30, 87.50];
    
    open.extend(pattern_open);
    high.extend(pattern_high);
    low.extend(pattern_low);
    close.extend(pattern_close);
    
    let options = [5.0, 2.0, 3.0];

    // Step 1: Full calculation
    let inputs = [
        open.as_slice(),
        high.as_slice(),
        low.as_slice(),
        close.as_slice(),
    ];
    let (result, _) = match indicator(&inputs, &options, None) { 
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Result: {:?}", result);

    if let Some(patterns) = result.last().and_then(|opt| opt.as_ref()) {
        println!("Forecast type None - Patterns found:");
        for pattern in patterns {
            let pattern_info = pattern.get_info();
            println!("  - {} ({}), Bars: {}",
                pattern_info.full_name,
                pattern_info.japanese_name,
                pattern_info.bars);
        }
    }

    let (result, _) = match indicator(&inputs, &options, Some(ForcastType::BearishReversal)) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\n\nResult: {:?}", result);

    if let Some(patterns) = result.last().and_then(|opt| opt.as_ref()) {
        println!("Forecast type Specified - Patterns found:");
        for pattern in patterns {
            let pattern_info = pattern.get_info();
            println!("  - {} ({}), Bars: {}",
                pattern_info.full_name,
                pattern_info.japanese_name,
                pattern_info.bars);
        }
    }

    
}
