#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub type TA_Integer = i32;
pub type TA_Real = f64;

#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum TA_RetCode {
    TA_SUCCESS = 0,
    TA_LIB_NOT_INITIALIZE = 1,
    TA_BAD_PARAM = 2,
    TA_ALLOC_ERR = 3,
    TA_GROUP_NOT_FOUND = 4,
    TA_FUNC_NOT_FOUND = 5,
    TA_INVALID_HANDLE = 6,
    TA_INVALID_PARAM_HOLDER = 7,
    TA_INVALID_PARAM_HOLDER_TYPE = 8,
    TA_INVALID_PARAM_FUNCTION = 9,
    TA_INPUT_NOT_ALL_INITIALIZE = 10,
    TA_OUTPUT_NOT_ALL_INITIALIZE = 11,
    TA_OUT_OF_RANGE_START_INDEX = 12,
    TA_OUT_OF_RANGE_END_INDEX = 13,
    TA_INVALID_LIST_TYPE = 14,
    TA_BAD_OBJECT = 15,
    TA_NOT_SUPPORTED = 16,
    TA_INTERNAL_ERROR = 5000,
}

extern "C" {
    // Exponential Moving Average (EMA)
    pub fn TA_EMA_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_EMA(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Simple Moving Average (SMA)
    pub fn TA_SMA_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_SMA(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Double Exponential Moving Average (DEMA)
    pub fn TA_DEMA_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_DEMA(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Triple Exponential Moving Average (TEMA)
    pub fn TA_TEMA_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_TEMA(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Triangular Moving Average (TRIMA)
    pub fn TA_TRIMA_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_TRIMA(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Weighted Moving Average (WMA)
    pub fn TA_WMA_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_WMA(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Kaufman Adaptive Moving Average (KAMA)
    pub fn TA_KAMA_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_KAMA(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // MACD - Moving Average Convergence/Divergence
    pub fn TA_MACD_Lookback(
        fast_period: TA_Integer,
        slow_period: TA_Integer,
        signal_period: TA_Integer,
    ) -> TA_Integer;
    pub fn TA_MACD(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        fast_period: TA_Integer,
        slow_period: TA_Integer,
        signal_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_macd: *mut TA_Real,
        out_macd_signal: *mut TA_Real,
        out_macd_hist: *mut TA_Real,
    ) -> TA_RetCode;

    // RSI - Relative Strength Index
    pub fn TA_RSI_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_RSI(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // ADX - Average Directional Movement Index
    pub fn TA_ADX_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_ADX(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // ADXR - Average Directional Movement Index Rating
    pub fn TA_ADXR_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_ADXR(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // ATR - Average True Range
    pub fn TA_ATR_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_ATR(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Volume Weighted Moving Average (VWMA)
    // Note: TA-Lib doesn't have native VWMA, but we can implement using weighted calculation

    // Wilders Smoothing
    // Note: TA-Lib doesn't have native WILDERS, equivalent to EMA with alpha = 1/period

    // Zero Lag Exponential Moving Average (ZLEMA)
    // Note: ZLEMA uses EMA lookback

    // Bollinger Bands
    pub fn TA_BBANDS_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_BBANDS(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        nb_dev_up: TA_Real,
        nb_dev_dn: TA_Real,
        ma_type: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real_upper_band: *mut TA_Real,
        out_real_middle_band: *mut TA_Real,
        out_real_lower_band: *mut TA_Real,
    ) -> TA_RetCode;

    // Stochastic Oscillator
    pub fn TA_STOCH_Lookback(
        fastk_period: TA_Integer,
        slowk_period: TA_Integer,
        slowk_matype: TA_Integer,
        slowd_period: TA_Integer,
        slowd_matype: TA_Integer,
    ) -> TA_Integer;
    pub fn TA_STOCH(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        fastk_period: TA_Integer,
        slowk_period: TA_Integer,
        slowk_matype: TA_Integer,
        slowd_period: TA_Integer,
        slowd_matype: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_slowk: *mut TA_Real,
        out_slowd: *mut TA_Real,
    ) -> TA_RetCode;

    // Commodity Channel Index (CCI)
    pub fn TA_CCI_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_CCI(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Rate of Change (ROC)
    pub fn TA_ROC_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_ROC(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Rate of Change Ratio (ROCR)
    pub fn TA_ROCR_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_ROCR(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Momentum (MOM)
    pub fn TA_MOM_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_MOM(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Williams %R
    pub fn TA_WILLR_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_WILLR(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // True Range (TR)
    pub fn TA_TRANGE_Lookback() -> TA_Integer;
    pub fn TA_TRANGE(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Standard Deviation
    pub fn TA_STDDEV_Lookback(time_period: TA_Integer, nb_dev: TA_Real) -> TA_Integer;
    pub fn TA_STDDEV(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        nb_dev: TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // On Balance Volume (OBV)
    pub fn TA_OBV_Lookback() -> TA_Integer;
    pub fn TA_OBV(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        in_volume: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Average Price
    pub fn TA_AVGPRICE_Lookback() -> TA_Integer;
    pub fn TA_AVGPRICE(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_open: *const TA_Real,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Median Price
    pub fn TA_MEDPRICE_Lookback() -> TA_Integer;
    pub fn TA_MEDPRICE(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Typical Price
    pub fn TA_TYPPRICE_Lookback() -> TA_Integer;
    pub fn TA_TYPPRICE(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Weighted Close Price
    pub fn TA_WCLPRICE_Lookback() -> TA_Integer;
    pub fn TA_WCLPRICE(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // MAX - Highest value over a specified period
    pub fn TA_MAX_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_MAX(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // MIN - Lowest value over a specified period
    pub fn TA_MIN_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_MIN(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // SUM - Summation
    pub fn TA_SUM_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_SUM(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Aroon
    pub fn TA_AROON_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_AROON(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_aroon_down: *mut TA_Real,
        out_aroon_up: *mut TA_Real,
    ) -> TA_RetCode;

    // Aroon Oscillator
    pub fn TA_AROONOSC_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_AROONOSC(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Ultimate Oscillator
    pub fn TA_ULTOSC_Lookback(
        time_period1: TA_Integer,
        time_period2: TA_Integer,
        time_period3: TA_Integer,
    ) -> TA_Integer;
    pub fn TA_ULTOSC(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        time_period1: TA_Integer,
        time_period2: TA_Integer,
        time_period3: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Absolute Price Oscillator
    pub fn TA_APO_Lookback(
        fast_period: TA_Integer,
        slow_period: TA_Integer,
        ma_type: TA_Integer,
    ) -> TA_Integer;
    pub fn TA_APO(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        fast_period: TA_Integer,
        slow_period: TA_Integer,
        ma_type: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Percentage Price Oscillator
    pub fn TA_PPO_Lookback(
        fast_period: TA_Integer,
        slow_period: TA_Integer,
        ma_type: TA_Integer,
    ) -> TA_Integer;
    pub fn TA_PPO(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        fast_period: TA_Integer,
        slow_period: TA_Integer,
        ma_type: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Chande Momentum Oscillator
    pub fn TA_CMO_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_CMO(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Stochastic RSI
    pub fn TA_STOCHRSI_Lookback(
        time_period: TA_Integer,
        fastk_period: TA_Integer,
        fastd_period: TA_Integer,
        fastd_matype: TA_Integer,
    ) -> TA_Integer;
    pub fn TA_STOCHRSI(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        fastk_period: TA_Integer,
        fastd_period: TA_Integer,
        fastd_matype: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_fastk: *mut TA_Real,
        out_fastd: *mut TA_Real,
    ) -> TA_RetCode;

    // Balance of Power
    pub fn TA_BOP_Lookback() -> TA_Integer;
    pub fn TA_BOP(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_open: *const TA_Real,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Money Flow Index
    pub fn TA_MFI_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_MFI(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        in_volume: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Accumulation/Distribution Line
    pub fn TA_AD_Lookback() -> TA_Integer;
    pub fn TA_AD(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        in_volume: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Linear Regression
    pub fn TA_LINEARREG_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_LINEARREG(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Linear Regression Slope
    pub fn TA_LINEARREG_SLOPE_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_LINEARREG_SLOPE(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Time Series Forecast
    pub fn TA_TSF_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_TSF(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Parabolic SAR
    pub fn TA_SAR_Lookback(acceleration: TA_Real, maximum: TA_Real) -> TA_Integer;
    pub fn TA_SAR(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        acceleration: TA_Real,
        maximum: TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Normalized Average True Range (NATR)
    pub fn TA_NATR_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_NATR(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // TRIX - 1-day Rate-Of-Change (ROC) of a Triple Smooth EMA
    pub fn TA_TRIX_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_TRIX(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Detrended Price Oscillator (DPO)
    pub fn TA_DPO_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_DPO(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Accumulation/Distribution Oscillator
    pub fn TA_ADOSC_Lookback(fast_period: TA_Integer, slow_period: TA_Integer) -> TA_Integer;
    pub fn TA_ADOSC(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        in_volume: *const TA_Real,
        fast_period: TA_Integer,
        slow_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Variable Index Dynamic Average (VIDYA)
    pub fn TA_VAR_Lookback(time_period: TA_Integer, nb_dev: TA_Real) -> TA_Integer;
    pub fn TA_VAR(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_real: *const TA_Real,
        time_period: TA_Integer,
        nb_dev: TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Mean Deviation
    // Note: MD will use medprice pattern

    // Mass Index
    pub fn TA_MASS_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_MASS(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Ease of Movement
    pub fn TA_EMV_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_EMV(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_volume: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Negative Volume Index
    pub fn TA_NVI_Lookback() -> TA_Integer;
    pub fn TA_NVI(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_close: *const TA_Real,
        in_volume: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    // Positive Volume Index
    pub fn TA_PVI_Lookback() -> TA_Integer;
    pub fn TA_PVI(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_close: *const TA_Real,
        in_volume: *const TA_Real,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;
    // TA-Lib initialization and shutdown functions
    pub fn TA_Initialize() -> TA_RetCode;
    pub fn TA_Shutdown() -> TA_RetCode;
}

// Helper functions to match Tulip API pattern for single-input indicators
pub fn ta_ema_start(period: f64) -> i32 {
    unsafe { TA_EMA_Lookback(period as TA_Integer) }
}

pub fn ta_ema(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_EMA(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

pub fn ta_sma_start(period: f64) -> i32 {
    unsafe { TA_SMA_Lookback(period as TA_Integer) }
}

pub fn ta_sma(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_SMA(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

pub fn ta_dema_start(period: f64) -> i32 {
    unsafe { TA_DEMA_Lookback(period as TA_Integer) }
}

pub fn ta_dema(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_DEMA(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

pub fn ta_rsi_start(period: f64) -> i32 {
    unsafe { TA_RSI_Lookback(period as TA_Integer) }
}

pub fn ta_rsi(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_RSI(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Helper functions for multi-input indicators
pub fn ta_adx_start(period: f64) -> i32 {
    unsafe { TA_ADX_Lookback(period as TA_Integer) }
}

pub fn ta_adx(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_ADX(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

pub fn ta_atr_start(period: f64) -> i32 {
    unsafe { TA_ATR_Lookback(period as TA_Integer) }
}

pub fn ta_atr(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_ATR(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Multi-output helper for MACD
pub fn ta_macd_start(fast_period: f64, slow_period: f64, signal_period: f64) -> i32 {
    unsafe {
        TA_MACD_Lookback(
            fast_period as TA_Integer,
            slow_period as TA_Integer,
            signal_period as TA_Integer,
        )
    }
}

pub fn ta_macd(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let macd_output = *outputs.offset(0);
        let signal_output = *outputs.offset(1);
        let hist_output = *outputs.offset(2);

        let fast_period = *options.offset(0) as TA_Integer;
        let slow_period = *options.offset(1) as TA_Integer;
        let signal_period = *options.offset(2) as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_MACD(
            0,
            size - 1,
            input_data,
            fast_period,
            slow_period,
            signal_period,
            &mut out_begin,
            &mut out_nb_element,
            macd_output,
            signal_output,
            hist_output,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Bollinger Bands
pub fn ta_bbands_start(period: f64) -> i32 {
    // Initialize TA-Lib if not already done
    unsafe { TA_Initialize() };

    // Manual calculation since TA_BBANDS_Lookback appears to be broken in this TA-Lib version
    // BBands lookback should be period - 1 for valid periods >= 2
    let period_int = period as TA_Integer;
    if period_int >= 2 {
        period_int - 1
    } else {
        -1 // Invalid period
    }
}

pub fn ta_bbands(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let upper_output = *outputs.offset(0);
        let middle_output = *outputs.offset(1);
        let lower_output = *outputs.offset(2);

        let period = *options.offset(0) as TA_Integer;
        let nb_dev_up = *options.offset(1);
        let nb_dev_dn = *options.offset(1); // Use same deviation for both bands

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_BBANDS(
            0,
            size - 1,
            input_data,
            period,
            nb_dev_up,
            nb_dev_dn,
            0, // SMA
            &mut out_begin,
            &mut out_nb_element,
            upper_output,
            middle_output,
            lower_output,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Stochastic Oscillator
pub fn ta_stoch_start(fastk_period: f64, slowk_period: f64, slowd_period: f64) -> i32 {
    unsafe {
        TA_STOCH_Lookback(
            fastk_period as TA_Integer,
            slowk_period as TA_Integer,
            0, // SMA
            slowd_period as TA_Integer,
            0, // SMA
        )
    }
}

pub fn ta_stoch(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let slowk_output = *outputs.offset(0);
        let slowd_output = *outputs.offset(1);

        let fastk_period = *options.offset(0) as TA_Integer;
        let slowk_period = *options.offset(1) as TA_Integer;
        let slowd_period = *options.offset(2) as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_STOCH(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            fastk_period,
            slowk_period,
            0, // SMA
            slowd_period,
            0, // SMA
            &mut out_begin,
            &mut out_nb_element,
            slowk_output,
            slowd_output,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Commodity Channel Index (CCI)
pub fn ta_cci_start(period: f64) -> i32 {
    unsafe { TA_CCI_Lookback(period as TA_Integer) }
}

pub fn ta_cci(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_CCI(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Rate of Change (ROC)
pub fn ta_roc_start(period: f64) -> i32 {
    unsafe { TA_ROC_Lookback(period as TA_Integer) }
}

pub fn ta_roc(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_ROC(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Rate of Change Ratio (ROCR)
pub fn ta_rocr_start(period: f64) -> i32 {
    unsafe { TA_ROCR_Lookback(period as TA_Integer) }
}

pub fn ta_rocr(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_ROCR(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Momentum (MOM)
pub fn ta_mom_start(period: f64) -> i32 {
    unsafe { TA_MOM_Lookback(period as TA_Integer) }
}

pub fn ta_mom(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_MOM(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Williams %R
pub fn ta_willr_start(period: f64) -> i32 {
    unsafe { TA_WILLR_Lookback(period as TA_Integer) }
}

pub fn ta_willr(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_WILLR(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// True Range (TR)
pub fn ta_tr_start() -> i32 {
    unsafe { TA_TRANGE_Lookback() }
}

pub fn ta_tr(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_TRANGE(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Standard Deviation
pub fn ta_stddev_start(period: f64) -> i32 {
    unsafe { TA_STDDEV_Lookback(period as TA_Integer, 1.0) }
}

pub fn ta_stddev(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_STDDEV(
            0,
            size - 1,
            input_data,
            period,
            1.0, // nb_dev
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// On Balance Volume (OBV)
pub fn ta_obv_start() -> i32 {
    unsafe { TA_OBV_Lookback() }
}

pub fn ta_obv(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let price_data = *inputs.offset(0);
        let volume_data = *inputs.offset(1);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_OBV(
            0,
            size - 1,
            price_data,
            volume_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Average Price
pub fn ta_avgprice_start() -> i32 {
    unsafe { TA_AVGPRICE_Lookback() }
}

pub fn ta_avgprice(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let open_data = *inputs.offset(0);
        let high_data = *inputs.offset(1);
        let low_data = *inputs.offset(2);
        let close_data = *inputs.offset(3);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_AVGPRICE(
            0,
            size - 1,
            open_data,
            high_data,
            low_data,
            close_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Median Price
pub fn ta_medprice_start() -> i32 {
    unsafe { TA_MEDPRICE_Lookback() }
}

// Aroon
pub fn ta_aroon_start(period: f64) -> i32 {
    unsafe { TA_AROON_Lookback(period as TA_Integer) }
}

pub fn ta_aroon(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let aroon_down_output = *outputs.offset(0);
        let aroon_up_output = *outputs.offset(1);
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_AROON(
            0,
            size - 1,
            high_data,
            low_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            aroon_down_output,
            aroon_up_output,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Aroon Oscillator
pub fn ta_aroonosc_start(period: f64) -> i32 {
    unsafe { TA_AROONOSC_Lookback(period as TA_Integer) }
}

pub fn ta_aroonosc(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_AROONOSC(
            0,
            size - 1,
            high_data,
            low_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Ultimate Oscillator
pub fn ta_ultosc_start(period1: f64, period2: f64, period3: f64) -> i32 {
    unsafe {
        TA_ULTOSC_Lookback(
            period1 as TA_Integer,
            period2 as TA_Integer,
            period3 as TA_Integer,
        )
    }
}

pub fn ta_ultosc(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;

        let period1 = *options.offset(0) as TA_Integer;
        let period2 = *options.offset(1) as TA_Integer;
        let period3 = *options.offset(2) as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_ULTOSC(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            period1,
            period2,
            period3,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Absolute Price Oscillator
pub fn ta_apo_start(fast_period: f64, slow_period: f64) -> i32 {
    unsafe {
        TA_APO_Lookback(
            fast_period as TA_Integer,
            slow_period as TA_Integer,
            0, // SMA
        )
    }
}

pub fn ta_apo(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;

        let fast_period = *options.offset(0) as TA_Integer;
        let slow_period = *options.offset(1) as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_APO(
            0,
            size - 1,
            input_data,
            fast_period,
            slow_period,
            0, // SMA
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Percentage Price Oscillator
pub fn ta_ppo_start(fast_period: f64, slow_period: f64) -> i32 {
    unsafe {
        TA_PPO_Lookback(
            fast_period as TA_Integer,
            slow_period as TA_Integer,
            0, // SMA
        )
    }
}

pub fn ta_ppo(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;

        let fast_period = *options.offset(0) as TA_Integer;
        let slow_period = *options.offset(1) as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_PPO(
            0,
            size - 1,
            input_data,
            fast_period,
            slow_period,
            0, // SMA
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Chande Momentum Oscillator
pub fn ta_cmo_start(period: f64) -> i32 {
    unsafe { TA_CMO_Lookback(period as TA_Integer) }
}

pub fn ta_cmo(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_CMO(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Stochastic RSI
pub fn ta_stochrsi_start(time_period: f64, fastk_period: f64, fastd_period: f64) -> i32 {
    unsafe {
        TA_STOCHRSI_Lookback(
            time_period as TA_Integer,
            fastk_period as TA_Integer,
            fastd_period as TA_Integer,
            0, // SMA
        )
    }
}

pub fn ta_stochrsi(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let fastk_output = *outputs.offset(0);
        let fastd_output = *outputs.offset(1);

        let time_period = *options.offset(0) as TA_Integer;
        let fastk_period = *options.offset(1) as TA_Integer;
        let fastd_period = *options.offset(2) as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_STOCHRSI(
            0,
            size - 1,
            input_data,
            time_period,
            fastk_period,
            fastd_period,
            0, // SMA
            &mut out_begin,
            &mut out_nb_element,
            fastk_output,
            fastd_output,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Balance of Power
pub fn ta_bop_start() -> i32 {
    unsafe { TA_BOP_Lookback() }
}

pub fn ta_bop(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let open_data = *inputs.offset(0);
        let high_data = *inputs.offset(1);
        let low_data = *inputs.offset(2);
        let close_data = *inputs.offset(3);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_BOP(
            0,
            size - 1,
            open_data,
            high_data,
            low_data,
            close_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Money Flow Index
pub fn ta_mfi_start(period: f64) -> i32 {
    unsafe { TA_MFI_Lookback(period as TA_Integer) }
}

pub fn ta_mfi(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let volume_data = *inputs.offset(3);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_MFI(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            volume_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Accumulation/Distribution Line
pub fn ta_ad_start() -> i32 {
    unsafe { TA_AD_Lookback() }
}

pub fn ta_ad(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let volume_data = *inputs.offset(3);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_AD(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            volume_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Linear Regression
pub fn ta_linreg_start(period: f64) -> i32 {
    unsafe { TA_LINEARREG_Lookback(period as TA_Integer) }
}

pub fn ta_linreg(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_LINEARREG(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Normalized Average True Range (NATR)
pub fn ta_natr_start(period: f64) -> i32 {
    unsafe { TA_NATR_Lookback(period as TA_Integer) }
}

pub fn ta_natr(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_NATR(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// TRIX
pub fn ta_trix_start(period: f64) -> i32 {
    unsafe { TA_TRIX_Lookback(period as TA_Integer) }
}

pub fn ta_trix(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_TRIX(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Detrended Price Oscillator (DPO)
pub fn ta_dpo_start(period: f64) -> i32 {
    unsafe { TA_DPO_Lookback(period as TA_Integer) }
}

pub fn ta_dpo(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_DPO(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Accumulation/Distribution Oscillator
pub fn ta_adosc_start(fast_period: f64, slow_period: f64) -> i32 {
    unsafe { TA_ADOSC_Lookback(fast_period as TA_Integer, slow_period as TA_Integer) }
}

pub fn ta_adosc(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let volume_data = *inputs.offset(3);
        let output_data = *outputs;

        let fast_period = *options.offset(0) as TA_Integer;
        let slow_period = *options.offset(1) as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_ADOSC(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            volume_data,
            fast_period,
            slow_period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Mass Index
pub fn ta_mass_start(period: f64) -> i32 {
    unsafe { TA_MASS_Lookback(period as TA_Integer) }
}

pub fn ta_mass(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_MASS(
            0,
            size - 1,
            high_data,
            low_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Ease of Movement
pub fn ta_emv_start(period: f64) -> i32 {
    unsafe { TA_EMV_Lookback(period as TA_Integer) }
}

pub fn ta_emv(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let volume_data = *inputs.offset(2);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_EMV(
            0,
            size - 1,
            high_data,
            low_data,
            volume_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Negative Volume Index
pub fn ta_nvi_start() -> i32 {
    unsafe { TA_NVI_Lookback() }
}

pub fn ta_nvi(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let close_data = *inputs.offset(0);
        let volume_data = *inputs.offset(1);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_NVI(
            0,
            size - 1,
            close_data,
            volume_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Positive Volume Index
pub fn ta_pvi_start() -> i32 {
    unsafe { TA_PVI_Lookback() }
}

pub fn ta_pvi(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let close_data = *inputs.offset(0);
        let volume_data = *inputs.offset(1);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_PVI(
            0,
            size - 1,
            close_data,
            volume_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// TEMA - Triple Exponential Moving Average
pub fn ta_tema_start(period: f64) -> i32 {
    unsafe { TA_TEMA_Lookback(period as TA_Integer) }
}

pub fn ta_tema(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_TEMA(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// TRIMA - Triangular Moving Average
pub fn ta_trima_start(period: f64) -> i32 {
    unsafe { TA_TRIMA_Lookback(period as TA_Integer) }
}

pub fn ta_trima(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_TRIMA(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// WMA - Weighted Moving Average
pub fn ta_wma_start(period: f64) -> i32 {
    unsafe { TA_WMA_Lookback(period as TA_Integer) }
}

pub fn ta_wma(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_WMA(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// HMA - Hull Moving Average

// KAMA - Kaufman Adaptive Moving Average
pub fn ta_kama_start(period: f64) -> i32 {
    unsafe { TA_KAMA_Lookback(period as TA_Integer) }
}

pub fn ta_kama(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_KAMA(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Linear Regression Slope
pub fn ta_linregslope_start(period: f64) -> i32 {
    unsafe { TA_LINEARREG_SLOPE_Lookback(period as TA_Integer) }
}

pub fn ta_linregslope(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_LINEARREG_SLOPE(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Time Series Forecast
pub fn ta_tsf_start(period: f64) -> i32 {
    unsafe { TA_TSF_Lookback(period as TA_Integer) }
}

pub fn ta_tsf(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_TSF(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Parabolic SAR
pub fn ta_psar_start(acceleration: f64, maximum: f64) -> i32 {
    unsafe { TA_SAR_Lookback(acceleration, maximum) }
}

pub fn ta_psar(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let output_data = *outputs;

        let acceleration = *options.offset(0);
        let maximum = *options.offset(1);

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_SAR(
            0,
            size - 1,
            high_data,
            low_data,
            acceleration,
            maximum,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

pub fn ta_medprice(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_MEDPRICE(
            0,
            size - 1,
            high_data,
            low_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Typical Price
pub fn ta_typprice_start() -> i32 {
    unsafe { TA_TYPPRICE_Lookback() }
}

pub fn ta_typprice(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_TYPPRICE(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// Weighted Close Price
pub fn ta_wclprice_start() -> i32 {
    unsafe { TA_WCLPRICE_Lookback() }
}

pub fn ta_wclprice(
    size: i32,
    inputs: *const *const f64,
    _options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_WCLPRICE(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// MAX - Highest value over a specified period
pub fn ta_max_start(period: f64) -> i32 {
    unsafe { TA_MAX_Lookback(period as TA_Integer) }
}

pub fn ta_max(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_MAX(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// MIN - Lowest value over a specified period
pub fn ta_min_start(period: f64) -> i32 {
    unsafe { TA_MIN_Lookback(period as TA_Integer) }
}

pub fn ta_min(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_MIN(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// SUM - Summation
pub fn ta_sum_start(period: f64) -> i32 {
    unsafe { TA_SUM_Lookback(period as TA_Integer) }
}

pub fn ta_sum(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let input_data = *inputs;
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_SUM(
            0,
            size - 1,
            input_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

pub fn ta_adxr_start(period: f64) -> i32 {
    unsafe { TA_ADXR_Lookback(period as TA_Integer) }
}

pub fn ta_adxr(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_ADXR(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

// PLUS_DI and MINUS_DI - Directional Indicators
extern "C" {
    pub fn TA_PLUS_DI_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_PLUS_DI(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;

    pub fn TA_MINUS_DI_Lookback(time_period: TA_Integer) -> TA_Integer;
    pub fn TA_MINUS_DI(
        start_idx: TA_Integer,
        end_idx: TA_Integer,
        in_high: *const TA_Real,
        in_low: *const TA_Real,
        in_close: *const TA_Real,
        time_period: TA_Integer,
        out_begin_idx: *mut TA_Integer,
        out_nb_element: *mut TA_Integer,
        out_real: *mut TA_Real,
    ) -> TA_RetCode;
}

pub fn ta_plus_di_start(period: f64) -> i32 {
    unsafe { TA_PLUS_DI_Lookback(period as TA_Integer) }
}

pub fn ta_plus_di(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_PLUS_DI(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}

pub fn ta_minus_di_start(period: f64) -> i32 {
    unsafe { TA_MINUS_DI_Lookback(period as TA_Integer) }
}

pub fn ta_minus_di(
    size: i32,
    inputs: *const *const f64,
    options: *const f64,
    outputs: *mut *mut f64,
) -> i32 {
    unsafe {
        let high_data = *inputs.offset(0);
        let low_data = *inputs.offset(1);
        let close_data = *inputs.offset(2);
        let output_data = *outputs;
        let period = *options as TA_Integer;

        let mut out_begin = 0;
        let mut out_nb_element = 0;

        let result = TA_MINUS_DI(
            0,
            size - 1,
            high_data,
            low_data,
            close_data,
            period,
            &mut out_begin,
            &mut out_nb_element,
            output_data,
        );

        match result {
            TA_RetCode::TA_SUCCESS => 0,
            _ => -1,
        }
    }
}
