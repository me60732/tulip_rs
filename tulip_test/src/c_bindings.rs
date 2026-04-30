#![allow(non_camel_case_types)]
pub type TI_REAL = f64;

extern "C" {
    // Triangular Moving Average (TRIMA)
    pub fn ti_trima_start(options: *const TI_REAL) -> i32;
    pub fn ti_trima(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Triple Exponential Moving Average (TEMA)
    pub fn ti_tema_start(options: *const TI_REAL) -> i32;
    pub fn ti_tema(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // TRIX
    pub fn ti_trix_start(options: *const TI_REAL) -> i32;
    pub fn ti_trix(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Exponential Moving Average (EMA)
    pub fn ti_ema_start(options: *const TI_REAL) -> i32;
    pub fn ti_ema(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Double Exponential Moving Average (DEMA)
    pub fn ti_dema_start(options: *const TI_REAL) -> i32;
    pub fn ti_dema(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Simple Moving Average (SMA)
    pub fn ti_sma_start(options: *const TI_REAL) -> i32;
    pub fn ti_sma(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Average Directional Movement Index (ADX)
    pub fn ti_adx_start(options: *const TI_REAL) -> i32;
    pub fn ti_adx(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Average Directional Movement Rating (ADXR)
    pub fn ti_adxr_start(options: *const TI_REAL) -> i32;
    pub fn ti_adxr(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Average True Range (ATR)
    pub fn ti_atr_start(options: *const TI_REAL) -> i32;
    pub fn ti_atr(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Directional Indicator (DI)
    pub fn ti_di_start(options: *const TI_REAL) -> i32;
    pub fn ti_di(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // On Balance Volume (OBV)
    pub fn ti_obv_start(options: *const TI_REAL) -> i32;
    pub fn ti_obv(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Commodity Channel Index (CCI)
    pub fn ti_cci_start(options: *const TI_REAL) -> i32;
    pub fn ti_cci(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Detrended Price Oscillator (DPO)
    pub fn ti_dpo_start(options: *const TI_REAL) -> i32;
    pub fn ti_dpo(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Fosc (Force Oscillator, if available)
    pub fn ti_fosc_start(options: *const TI_REAL) -> i32;
    pub fn ti_fosc(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Klinger Volume Oscillator (KVO)
    pub fn ti_kvo_start(options: *const TI_REAL) -> i32;
    pub fn ti_kvo(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Linear Regression (LINREG)
    pub fn ti_linreg_start(options: *const TI_REAL) -> i32;
    pub fn ti_linreg(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
     // Linear Intercept (LINREG)
     pub fn ti_linregintercept_start(options: *const TI_REAL) -> i32;
     pub fn ti_linregintercept(
         size: i32,
         inputs: *const *const TI_REAL,
         options: *const TI_REAL,
         outputs: *mut *mut TI_REAL,
     ) -> i32;
 // Linear Regression (LINREG)
    pub fn ti_linregslope_start(options: *const TI_REAL) -> i32;
    pub fn ti_linregslope(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    // Percentage Volume Index (PVI)
    pub fn ti_pvi_start(options: *const TI_REAL) -> i32;
    pub fn ti_pvi(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Percentage Price Oscillator (PPO)
    pub fn ti_ppo_start(options: *const TI_REAL) -> i32;
    pub fn ti_ppo(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Parabolic SAR (PSAR)
    pub fn ti_psar_start(options: *const TI_REAL) -> i32;
    pub fn ti_psar(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // QStick
    pub fn ti_qstick_start(options: *const TI_REAL) -> i32;
    pub fn ti_qstick(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Rate of Change (ROC)
    pub fn ti_roc_start(options: *const TI_REAL) -> i32;
    pub fn ti_roc(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Rate of Change Ratio (ROCR)
    pub fn ti_rocr_start(options: *const TI_REAL) -> i32;
    pub fn ti_rocr(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;

    // Relative Strength Index (RSI)
    pub fn ti_rsi_start(options: *const TI_REAL) -> i32;
    pub fn ti_rsi(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    // MACD
    pub fn ti_macd_start(options: *const TI_REAL) -> i32;
    pub fn ti_macd(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    // HMA
    pub fn ti_hma_start(options: *const TI_REAL) -> i32;
    pub fn ti_hma(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //kama
    pub fn ti_kama_start(options: *const TI_REAL) -> i32;
    pub fn ti_kama(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //ad
    pub fn ti_ad_start(options: *const TI_REAL) -> i32;
    pub fn ti_ad(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //adosc
    pub fn ti_adosc_start(options: *const TI_REAL) -> i32;
    pub fn ti_adosc(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //ao
    pub fn ti_ao_start(options: *const TI_REAL) -> i32;
    pub fn ti_ao(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //apo
    pub fn ti_apo_start(options: *const TI_REAL) -> i32;
    pub fn ti_apo(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //aroon
    pub fn ti_aroon_start(options: *const TI_REAL) -> i32;
    pub fn ti_aroon(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //aroon
    pub fn ti_aroonosc_start(options: *const TI_REAL) -> i32;
    pub fn ti_aroonosc(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //natr
    pub fn ti_natr_start(options: *const TI_REAL) -> i32;
    pub fn ti_natr(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //avgprice
    pub fn ti_avgprice_start(options: *const TI_REAL) -> i32;
    pub fn ti_avgprice(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //bbands
    pub fn ti_bbands_start(options: *const TI_REAL) -> i32;
    pub fn ti_bbands(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //bop
    pub fn ti_bop_start(options: *const TI_REAL) -> i32;
    pub fn ti_bop(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //cmo
    pub fn ti_cmo_start(options: *const TI_REAL) -> i32;
    pub fn ti_cmo(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //cvi
    pub fn ti_cvi_start(options: *const TI_REAL) -> i32;
    pub fn ti_cvi(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
     //dm
     pub fn ti_dm_start(options: *const TI_REAL) -> i32;
     pub fn ti_dm(
         size: i32,
         inputs: *const *const TI_REAL,
         options: *const TI_REAL,
         outputs: *mut *mut TI_REAL,
     ) -> i32;
      //dx
      pub fn ti_dx_start(options: *const TI_REAL) -> i32;
      pub fn ti_dx(
          size: i32,
          inputs: *const *const TI_REAL,
          options: *const TI_REAL,
          outputs: *mut *mut TI_REAL,
      ) -> i32;
    //emv
    pub fn ti_emv_start(options: *const TI_REAL) -> i32;
    pub fn ti_emv(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //marketfi
    pub fn ti_marketfi_start(options: *const TI_REAL) -> i32;
    pub fn ti_marketfi(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
     //mass
     pub fn ti_mass_start(options: *const TI_REAL) -> i32;
     pub fn ti_mass(
         size: i32,
         inputs: *const *const TI_REAL,
         options: *const TI_REAL,
         outputs: *mut *mut TI_REAL,
     ) -> i32;
    //max
    pub fn ti_max_start(options: *const TI_REAL) -> i32;
    pub fn ti_max(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //min
    pub fn ti_min_start(options: *const TI_REAL) -> i32;
    pub fn ti_min(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //md
    pub fn ti_md_start(options: *const TI_REAL) -> i32;
    pub fn ti_md(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //md
    pub fn ti_medprice_start(options: *const TI_REAL) -> i32;
    pub fn ti_medprice(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //mfi
    pub fn ti_mfi_start(options: *const TI_REAL) -> i32;
    pub fn ti_mfi(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //mfi
    pub fn ti_mom_start(options: *const TI_REAL) -> i32;
    pub fn ti_mom(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //msw
    pub fn ti_msw_start(options: *const TI_REAL) -> i32;
    pub fn ti_msw(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //nvi
    pub fn ti_nvi_start(options: *const TI_REAL) -> i32;
    pub fn ti_nvi(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //stddev
    pub fn ti_stddev_start(options: *const TI_REAL) -> i32;
    pub fn ti_stddev(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //stoch
    pub fn ti_stoch_start(options: *const TI_REAL) -> i32;
    pub fn ti_stoch(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //stochrsi
    pub fn ti_stochrsi_start(options: *const TI_REAL) -> i32;
    pub fn ti_stochrsi(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //tsf
    pub fn ti_tsf_start(options: *const TI_REAL) -> i32;
    pub fn ti_tsf(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //typprice
    pub fn ti_typprice_start(options: *const TI_REAL) -> i32;
    pub fn ti_typprice(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
     //ultosc
     pub fn ti_ultosc_start(options: *const TI_REAL) -> i32;
     pub fn ti_ultosc(
         size: i32,
         inputs: *const *const TI_REAL,
         options: *const TI_REAL,
         outputs: *mut *mut TI_REAL,
     ) -> i32;
    //ultosc
    pub fn ti_vhf_start(options: *const TI_REAL) -> i32;
    pub fn ti_vhf(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //vidya
    pub fn ti_vidya_start(options: *const TI_REAL) -> i32;
    pub fn ti_vidya(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //volatility
    pub fn ti_volatility_start(options: *const TI_REAL) -> i32;
    pub fn ti_volatility(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //vosc
    pub fn ti_vosc_start(options: *const TI_REAL) -> i32;
    pub fn ti_vosc(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //vwma
    pub fn ti_vwma_start(options: *const TI_REAL) -> i32;
    pub fn ti_vwma(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //wad
    pub fn ti_wad_start(options: *const TI_REAL) -> i32;
    pub fn ti_wad(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //wcprice
    pub fn ti_wcprice_start(options: *const TI_REAL) -> i32;
    pub fn ti_wcprice(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //wilders
    pub fn ti_wilders_start(options: *const TI_REAL) -> i32;
    pub fn ti_wilders(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //willr
    pub fn ti_willr_start(options: *const TI_REAL) -> i32;
    pub fn ti_willr(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //wma
    pub fn ti_wma_start(options: *const TI_REAL) -> i32;
    pub fn ti_wma(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //zlema
    pub fn ti_zlema_start(options: *const TI_REAL) -> i32;
    pub fn ti_zlema(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    //tr
    pub fn ti_tr_start(options: *const TI_REAL) -> i32;
    pub fn ti_tr(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
    
    // fisher
    pub fn ti_fisher_start(options: *const TI_REAL) -> i32;
    pub fn ti_fisher(
        size: i32,
        inputs: *const *const TI_REAL,
        options: *const TI_REAL,
        outputs: *mut *mut TI_REAL,
    ) -> i32;
}