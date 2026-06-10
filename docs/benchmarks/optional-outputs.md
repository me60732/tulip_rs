# Optional Outputs

`tulip_rs` computes the primary indicator **and** all intermediate outputs in a single pass through the data. C Tulip and TA-Lib require separate function calls for each intermediate result.

The **Equiv. Total** column sums the competitor's time for the primary indicator plus each sub-indicator called separately. The Rust time already includes every optional output.

=== "vs C Tulip"

    | Indicator | Optional outputs | Rust all-outputs (ns) | C Equiv. Total (ns) | C / Rust |
    |-----------|-----------------|----------------------:|--------------------:|---------:|
    | adosc | short_ema, long_ema, ad | 7,843 | 35,053 | **4.47×** |
    | adx | dx, atr, tr | 10,474 | 32,055 | **3.06×** |
    | adxr | adx, dx, atr, tr | 17,064 | 54,662 | **3.20×** |
    | ao | short_sma, long_sma, medprice | 10,006 | 18,285 | **1.83×** |
    | apo | short_ema, long_ema | 7,204 | 29,575 | **4.11×** |
    | aroonosc | aroon_down, aroon_up | 21,484 | 72,709 | **3.38×** |
    | atr | tr | 4,599 | 12,439 | **2.70×** |
    | cci | sma, md, typprice | 55,995 | 91,255 | **1.63×** |
    | dema | ema | 6,393 | 16,827 | **2.63×** |
    | di | atr, tr | 10,978 | 22,506 | **2.05×** |
    | dpo | sma | 3,457 | 5,224 | **1.51×** |
    | dx | atr, tr | 9,293 | 18,806 | **2.02×** |
    | elderray | ema | 7,409 | 25,189 | **3.40×** |
    | emv | medprice | 3,521 | 6,436 | **1.83×** |
    | fosc | tsf, linreg, linregslope, linregintercept | 9,726 | 25,616 | **2.63×** |
    | kvo | short_ema, long_ema | 8,902 | 32,275 | **3.63×** |
    | linreg | linregslope, linregintercept | 6,239 | 16,438 | **2.63×** |
    | macd | short_ema, long_ema | 11,169 | 31,709 | **2.84×** |
    | md | sma | 12,339 | 16,864 | **1.37×** |
    | mfi | typprice | 8,629 | 18,954 | **2.20×** |
    | natr | atr, tr | 5,476 | 22,950 | **4.19×** |
    | ppo | short_ema, long_ema | 4,878 | 30,177 | **6.19×** |
    | roc | mom | 2,861 | 3,970 | **1.39×** |
    | stddev | sma | 3,908 | 12,955 | **3.31×** |
    | stochrsi | rsi | 19,190 | 52,502 | **2.74×** |
    | tema | dema, ema | 7,751 | 23,602 | **3.05×** |
    | trix | tema, dema, ema | 8,799 | 34,121 | **3.88×** |
    | tsf | tsf, linreg, linregslope, linregintercept | 7,810 | 24,663 | **3.16×** |
    | vidya | short_sma, long_sma, short_stddev, long_stddev | 12,016 | 44,226 | **3.68×** |
    | vosc | short_sma, long_sma | 7,466 | 9,994 | **1.34×** |
    | wma | sma | 5,492 | 10,871 | **1.98×** |

    *Indicators with no C Tulip equivalent (chandelierexit, keltnerchannel, trvi, vortex) are not shown.*

=== "vs TA-Lib"

    Only indicators where TA-Lib implements at least one equivalent sub-indicator are shown.

    | Indicator | Optional outputs | Rust all-outputs (ns) | TA-Lib Equiv. Total (ns) | TA-Lib / Rust |
    |-----------|-----------------|----------------------:|-------------------------:|--------------:|
    | adosc | short_ema, long_ema, ad | 7,843 | 34,504 | **4.40×** |
    | adx | dx, atr, tr | 10,474 | 67,869 | **6.48×** |
    | adxr | adx, dx, atr, tr | 17,064 | 105,705 | **6.19×** |
    | apo | short_ema, long_ema | 7,204 | 32,007 | **4.44×** |
    | aroonosc | aroon_down, aroon_up | 21,484 | 140,135 | **6.52×** |
    | atr | tr | 4,599 | 31,134 | **6.77×** |
    | cci | sma, md, typprice | 55,995 | 123,342 | **2.20×** |
    | dema | ema | 6,393 | 32,741 | **5.12×** |
    | di | atr, tr | 10,978 | 85,756 | **7.81×** |
    | macd | short_ema, long_ema | 11,169 | 56,816 | **5.09×** |
    | mfi | typprice | 8,629 | 37,618 | **4.36×** |
    | natr | atr, tr | 5,476 | 58,470 | **10.68×** |
    | ppo | short_ema, long_ema | 4,878 | 34,915 | **7.16×** |
    | roc | mom | 2,861 | 6,609 | **2.31×** |
    | tema | dema, ema | 7,751 | 64,748 | **8.35×** |
    | wma | sma | 5,492 | 9,602 | **1.75×** |

??? info "C Tulip calculation details"

    C Tulip times taken from the Standard benchmark (single call per sub-indicator):

    | # | Indicator | Calculation |
    |---|-----------|-------------|
    | adosc C | C | adosc(9076) + ema(10501) + ema(10501) + ad(4975) = **35,053** |
    | adosc TA | TA-Lib | adosc(8556) + ema(10477) + ema(10477) + ad(4994) = **34,504** |
    | adx C | C | adx(13249) + dx(6367) + atr(10439) + tr(2000) = **32,055** |
    | adx TA | TA-Lib | adx(36735) + atr(27396) + tr(3738) = **67,869** |
    | adxr C | C | adxr(22607) + adx(13249) + dx(6367) + atr(10439) + tr(2000) = **54,662** |
    | adxr TA | TA-Lib | adxr(37836) + adx(36735) + atr(27396) + tr(3738) = **105,705** |
    | ao C | C | ao(11548) + sma(2572) + sma(2572) + medprice(1593) = **18,285** |
    | apo C | C | apo(8573) + ema(10501) + ema(10501) = **29,575** |
    | apo TA | TA-Lib | apo(11053) + ema(10477) + ema(10477) = **32,007** |
    | aroonosc C | C | aroonosc(35588) + aroon(37121) = **72,709** *(aroon returns both up and down in one call)* |
    | aroonosc TA | TA-Lib | aroonosc(68045) + aroon(72090) = **140,135** *(aroon returns both up and down in one call)* |
    | atr C | C | atr(10439) + tr(2000) = **12,439** |
    | atr TA | TA-Lib | atr(27396) + tr(3738) = **31,134** |
    | cci C | C | cci(72586) + sma(2572) + md(14292) + typprice(1805) = **91,255** *(md and typprice not available separately in TA-Lib)* |
    | cci TA | TA-Lib | cci(118567) + sma(4775) = **123,342** *(md and typprice not available separately in TA-Lib)* |
    | dema C | C | dema(6326) + ema(10501) = **16,827** |
    | dema TA | TA-Lib | dema(22264) + ema(10477) = **32,741** |
    | di C | C | di(10067) + atr(10439) + tr(2000) = **22,506** |
    | di TA | TA-Lib | di(54622) + atr(27396) + tr(3738) = **85,756** |
    | dpo C | C | dpo(2652) + sma(2572) = **5,224** |
    | dx C | C | dx(6367) + atr(10439) + tr(2000) = **18,806** |
    | elderray C | C | elderray(14688) + ema(10501) = **25,189** |
    | emv C | C | emv(4843) + medprice(1593) = **6,436** |
    | fosc C | C | fosc(9175) + tsf(8222) + linreg(8219) = **25,616** *(slope/intercept have no standalone C equivalent)* |
    | kvo C | C | kvo(11273) + ema(10501) + ema(10501) = **32,275** |
    | linreg C | C | linreg(8219) + linreg(8219) = **16,438** *(slope and intercept not available as standalone in C Tulip)* |
    | macd C | C | macd(10707) + ema(10501) + ema(10501) = **31,709** |
    | macd TA | TA-Lib | macd(35862) + ema(10477) + ema(10477) = **56,816** |
    | md C | C | md(14292) + sma(2572) = **16,864** |
    | mfi C | C | mfi(17149) + typprice(1805) = **18,954** *(typprice not available separately in TA-Lib)* |
    | mfi TA | TA-Lib | mfi(18809) + mfi(18809) = **37,618** *(typprice not available separately in TA-Lib)* |
    | natr C | C | natr(10511) + atr(10439) + tr(2000) = **22,950** |
    | natr TA | TA-Lib | natr(27336) + atr(27396) + tr(3738) = **58,470** |
    | ppo C | C | ppo(9175) + ema(10501) + ema(10501) = **30,177** |
    | ppo TA | TA-Lib | ppo(13961) + ema(10477) + ema(10477) = **34,915** |
    | roc C | C | roc(2596) + mom(1374) = **3,970** |
    | roc TA | TA-Lib | roc(4796) + mom(1813) = **6,609** |
    | stddev C | C | stddev(10383) + sma(2572) = **12,955** |
    | stochrsi C | C | stochrsi(43358) + rsi(9144) = **52,502** |
    | tema C | C | tema(6775) + dema(6326) + ema(10501) = **23,602** |
    | tema TA | TA-Lib | tema(32007) + dema(22264) + ema(10477) = **64,748** |
    | trix C | C | trix(10519) + tema(6775) + dema(6326) + ema(10501) = **34,121** |
    | tsf C | C | tsf(8222) + tsf(8222) + linreg(8219) = **24,663** *(slope/intercept have no standalone C equivalent)* |
    | vidya C | C | vidya(18316) + sma(2572) + sma(2572) + stddev(10383) + stddev(10383) = **44,226** |
    | vosc C | C | vosc(4850) + sma(2572) + sma(2572) = **9,994** |
    | wma C | C | wma(8299) + sma(2572) = **10,871** |
    | wma TA | TA-Lib | wma(4827) + sma(4775) = **9,602** |

## Ranked by Advantage

| Rank | vs | Indicator | Ratio | Key sub-indicators |
|:----:|:--:|-----------|------:|--------------------|
| 1 | TA-Lib | `natr` | **10.68×** | atr, tr |
| 2 | TA-Lib | `tema` | **8.35×** | dema, ema |
| 3 | TA-Lib | `di` | **7.81×** | atr, tr |
| 4 | TA-Lib | `ppo` | **7.16×** | ema, ema |
| 5 | TA-Lib | `atr` | **6.77×** | tr |
| 6 | TA-Lib | `aroonosc` | **6.52×** | aroon |
| 7 | TA-Lib | `adx` | **6.48×** | atr, tr |
| 8 | TA-Lib | `adxr` | **6.19×** | adx, atr, tr |
| 9 | C Tulip | `ppo` | **6.19×** | ema, ema |
| 10 | TA-Lib | `dema` | **5.12×** | ema |
