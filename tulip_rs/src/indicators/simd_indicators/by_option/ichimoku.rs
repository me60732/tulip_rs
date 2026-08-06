use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::ichimoku::{
    validate_options as ichimoku_validate_options, Ichimoku, Indicator, IndicatorState, State,
    INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::ichimoku_simd::{options::SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// Per-lane periods tuple: `((short, short_look_back), (long, long_look_back), (ultra, ultra_look_back))`.
type Periods = ((usize, usize), (usize, usize), (usize, usize));

/// SIMD driver for the Ichimoku Cloud indicator, processing `N` option-set lanes per epoch.
struct IchimokuOptionsDriver;

impl Driver<State<Warm>, Periods> for IchimokuOptionsDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    ///
    /// Each lane independently uses its own `(short_look_back, long_look_back, ultra_look_back)`
    /// drawn from `options[lane]`. The shared input pointer array means all lanes operate on
    /// the same price series at per-lane bar indices tracked by `i_simd`.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&Periods>>,
    ) {
        let len = outputs[0][0].len();

        let (short_look_back, long_look_back, ultra_look_back, mut i_simd) = {
            let mut slb = [0usize; N];
            let mut llb = [0usize; N];
            let mut ulb = [0usize; N];
            let mut i_arr = [0usize; N];
            for (lane, opt) in options.iter().enumerate() {
                if let Some(&periods) = opt {
                    slb[lane] = periods.0 .1;
                    llb[lane] = periods.1 .1;
                    ulb[lane] = periods.2 .1;
                    // i starts at ultra_look_back; the raw pointer starts at
                    // data[current_epoch_start - ultra_look_back], so i_simd[lane] =
                    // ultra_look_back indexes the first bar of the current epoch.
                    i_arr[lane] = periods.2 .1;
                }
            }
            (
                Simd::from_array(slb),
                Simd::from_array(llb),
                Simd::from_array(ulb),
                Simd::from_array(i_arr),
            )
        };

        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);
        let (conv_ptr, base_ptr, span_a_ptr, span_b_ptr) =
            crate::extract_output_ptrs!(outputs, N, conv_ptr, base_ptr, span_a_ptr, span_b_ptr);

        let mut state = SimdState::from_states(&mut states);
        let one_splat = Simd::splat(1usize);

        for j in 0..len {
            let (conv, base, span_a, span_b) = state.calc((
                high_ptrs,
                low_ptrs,
                i_simd,
                short_look_back,
                long_look_back,
                ultra_look_back,
            ));
            crate::write_simd_at_indices!(N, j,
                conv_ptr => conv,
                base_ptr => base,
                span_a_ptr => span_a,
                span_b_ptr => span_b
            );
            i_simd += one_splat;
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Ichimoku Cloud on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// Output lengths are preserved: each lane's `conversion` and `base` include their earlier
/// bars (filled scalar by [`State::init_state`]), while `span_a` and `span_b` start at
/// the lane's ultra-long window boundary. The `lagging_span` (Chikou Span) is returned as
/// a copy of the close series when requested.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`),
///   containing `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane:
///   `[short_period, long_period]`.
/// * `optional_outputs` - Pass `Some(&[true])` to include `lagging_span`; `None` or
///   `Some(&[false])` disables it.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains
/// `[conversion, base, span_a, span_b, lagging_span]` and `states[i]` is the final
/// [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Ichimoku::min_data)?;
    validate_options(options, Some(ichimoku_validate_options))?;

    let [high, low, close] = *inputs;

    let want_lagging_span = optional_outputs
        .and_then(|oo| oo.first().copied())
        .unwrap_or(false);

    let params: [Periods; N] = std::array::from_fn(|i| {
        let short_period = options[i][0] as usize;
        let long_period = options[i][1] as usize;
        let ultra_long = long_period * 2;
        (
            (short_period, short_period - 1),
            (long_period, long_period - 1),
            (ultra_long, ultra_long - 1),
        )
    });

    let mut road_train = PrimeMover::<N, State<Warm>, Periods>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let periods = params[i];
        let ultra_look_back = periods.2 .1;

        let caps = Ichimoku::slot_lengths(high.len(), options[i]);

        let mut conversion_line = crate::uninit_vec!(f64, caps[0]);
        let mut base_line = crate::uninit_vec!(f64, caps[1]);
        let mut span_a_line = crate::uninit_vec!(f64, caps[2]);
        let span_b_line = crate::uninit_vec!(f64, caps[3]);

        let lagging_span = if want_lagging_span {
            close.to_vec()
        } else {
            Vec::with_capacity(0)
        };

        // Fills warmup bars of conversion/base/span_a and advances ring-buffer states.
        let state = State::init_state(
            (high, low),
            periods,
            (&mut conversion_line, &mut base_line, &mut span_a_line),
        );

        // Compute tail offsets: conversion and base have leading bars from init_state;
        // span_a and span_b are fully written by the driver (starts = 0).
        let mut starts = [0usize; 4];
        (starts[0], starts[1]) =
            crate::slice_outputs_start!(span_b_line.len(), conversion_line, base_line);

        let asset_inputs = vec![high, low];

        let mut output_buffer = vec![
            conversion_line,
            base_line,
            span_a_line,
            span_b_line,
            lagging_span,
        ];

        let mut asset_outputs = Vec::with_capacity(4);
        for j in 0..4 {
            unsafe {
                let buf = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    buf.as_mut_ptr().add(starts[j]),
                    buf.len() - starts[j],
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            ultra_look_back,
            ultra_look_back,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = IchimokuOptionsDriver;
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, &periods) in states_vec.into_iter().zip(params.iter()) {
        states.push(IndicatorState::new(high, low, periods, state));
    }
    Ok((output_buffers, states))
}
