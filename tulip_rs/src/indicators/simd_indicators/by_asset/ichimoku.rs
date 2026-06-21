use crate::common_simd::assets::validate_inputs;
use crate::indicators::ichimoku::{
    min_data, output_length, validate_options, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::ichimoku_simd::{assets::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;

/// SIMD driver that advances the Ichimoku Cloud across `N` asset lanes per scheduling epoch.
struct IchimokuDriver {
    short_look_back: usize,
    long_look_back: usize,
    ultra_look_back: usize,
}

impl Driver<State> for IchimokuDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (high) and `inputs[asset][1]` (low), writes all four
    /// Ichimoku output lines to `outputs[asset][0..4]`, and updates `states[asset]` in place.
    ///
    /// The driver expects each input slice to include `ultra_look_back` bars of leading context
    /// so that the min/max rescan can access the full window at the start of each epoch.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let data_len = inputs[0][0].len();

        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);
        let (conv_ptr, base_ptr, span_a_ptr, span_b_ptr) =
            crate::extract_output_ptrs!(outputs, N, conv_ptr, base_ptr, span_a_ptr, span_b_ptr);

        let mut state = SimdState::new(&mut states);
        let (slb, llb, ulb) = (
            self.short_look_back,
            self.long_look_back,
            self.ultra_look_back,
        );
        for (j, i) in (ulb..data_len).enumerate() {
            let (conv, base, span_a, span_b) = unsafe {
                state.calc_unchecked_simd::<1, 4, 4>(high_ptrs, low_ptrs, i, slb, llb, ulb)
            };
            crate::write_simd_at_indices!(N, j,
                conv_ptr => conv,
                base_ptr => base,
                span_a_ptr => span_a,
                span_b_ptr => span_b
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Ichimoku Cloud for `N` assets simultaneously using SIMD parallelism.
///
/// All assets share the same `options`. Uses the [`PrimeMover`] scheduler to batch assets
/// into SIMD-width groups.
///
/// Output lengths are preserved in full: `conversion` and `base` include their earlier bars
/// (filled scalar by [`State::init_state`] before the SIMD epoch begins), while `span_a` and
/// `span_b` start at the ultra-long window boundary. The `lagging_span` (Chikou Span) is
/// returned as a copy of each asset's close series when requested.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[short_period, long_period]`.
/// * `optional_outputs` - Pass `Some(&[true])` to include `lagging_span`; `None` or
///   `Some(&[false])` disables it.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains
/// `[conversion, base, span_a, span_b, lagging_span]` for asset `i` and `states[i]` is
/// the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;

    let want_lagging_span = optional_outputs
        .and_then(|oo| oo.first().copied())
        .unwrap_or(false);

    let periods = {
        let (short_period, long_period) = (options[0] as usize, options[1] as usize);
        let ultra_long = long_period * 2;
        (
            (short_period, short_period - 1),
            (long_period, long_period - 1),
            (ultra_long, ultra_long - 1),
        )
    };
    let (short_look_back, long_look_back, ultra_look_back) =
        (periods.0 .1, periods.1 .1, periods.2 .1);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let [high, low, close] = *inputs[i];
        let len = high.len();
        let (conversion_capacity, base_capacity, span_a_capacity, span_b_capacity, _) =
            output_length(len, options);

        let mut conversion_line = crate::uninit_vec!(f64, conversion_capacity);
        let mut base_line = crate::uninit_vec!(f64, base_capacity);
        let mut span_a_line = crate::uninit_vec!(f64, span_a_capacity);
        let span_b_line = crate::uninit_vec!(f64, span_b_capacity);

        let lagging_span = if want_lagging_span {
            close.to_vec()
        } else {
            Vec::with_capacity(0)
        };

        // Fills warmup bars of conversion/base/span_a (indices short_look_back..ultra_look_back)
        // and advances all six ring-buffer states through bar ultra_look_back - 1.
        let state = State::init_state(
            (high, low),
            periods,
            (&mut conversion_line, &mut base_line, &mut span_a_line),
        );

        // Compute where the SIMD driver begins writing inside each output buffer.
        // conversion and base have extra leading bars filled by init_state above;
        // span_a and span_b start at the ultra window boundary (starts = 0).
        let mut starts = [0usize; 4];
        (starts[0], starts[1]) =
            crate::slice_outputs_start!(span_b_capacity, conversion_line, base_line);

        let asset_inputs = vec![high, low];

        // output_buffer owns all five output vecs; asset_outputs gives the driver
        // the tail slices for the four computed outputs (lagging_span is excluded).
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

        // inputs_idx = ultra_look_back ensures the driver receives data[0..ultra_look_back+epoch]
        // each epoch, keeping the raw-pointer window correct for the ring-buffer rescan.
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            ultra_look_back,
            ultra_look_back,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = IchimokuDriver {
        short_look_back,
        long_look_back,
        ultra_look_back,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            inputs[i][0],
            inputs[i][1],
            periods,
            state,
        ));
    }
    Ok((output_buffers, states))
}
