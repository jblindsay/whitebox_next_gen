//! Adaptive arithmetic coding models used by LASzip-style codecs.

/// Number of fractional bits used by symbol CDF scaling.
pub const SYMBOL_LENGTH_SHIFT: u32 = 15;
/// Renormalization threshold for adaptive symbol counts.
pub const SYMBOL_MAX_COUNT: u32 = 1 << SYMBOL_LENGTH_SHIFT;

/// Number of fractional bits used by bit probability scaling.
pub const BIT_LENGTH_SHIFT: u32 = 13;
/// Renormalization threshold for adaptive bit counts.
pub const BIT_MAX_COUNT: u32 = 1 << BIT_LENGTH_SHIFT;

/// Adaptive model for binary symbols.
#[derive(Debug, Clone)]
pub struct ArithmeticBitModel {
    zero_count: u32,
    total_count: u32,
    zero_probability: u32,
    updates_until_refresh: u32,
    update_cycle: u32,
}

impl Default for ArithmeticBitModel {
    fn default() -> Self {
        Self {
            zero_count: 1,
            total_count: 2,
            zero_probability: 1u32 << (BIT_LENGTH_SHIFT - 1),
            updates_until_refresh: 4,
            update_cycle: 4,
        }
    }
}

impl ArithmeticBitModel {
    /// Create a bit model initialized to equiprobable state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return scaled probability for bit value zero.
    pub fn zero_probability(&self) -> u32 {
        self.zero_probability
    }

    /// Notify model that a bit was decoded/encoded.
    pub fn observe_bit(&mut self, bit: bool) {
        if !bit {
            self.zero_count += 1;
        }
        self.updates_until_refresh -= 1;
        if self.updates_until_refresh == 0 {
            self.refresh();
        }
    }

    fn refresh(&mut self) {
        self.total_count += self.update_cycle;
        if self.total_count > BIT_MAX_COUNT {
            self.total_count = (self.total_count + 1) >> 1;
            self.zero_count = (self.zero_count + 1) >> 1;
            if self.zero_count >= self.total_count {
                self.total_count = self.zero_count + 1;
            }
        }

        let scale = 0x8000_0000u32 / self.total_count;
        self.zero_probability = (self.zero_count * scale) >> (31 - BIT_LENGTH_SHIFT);

        self.update_cycle = (5 * self.update_cycle) >> 2;
        if self.update_cycle > 64 {
            self.update_cycle = 64;
        }
        self.updates_until_refresh = self.update_cycle;
    }
}

/// Adaptive model for symbols in range `[0, symbols)`.
#[derive(Debug, Clone)]
pub struct ArithmeticSymbolModel {
    symbols: u32,
    last_symbol: u32,
    counts: Vec<u32>,
    /// Monotonic scaled CDF values of length `symbols`.
    /// For symbol `i`, interval starts at `cdf[i]` and ends at `cdf[i + 1]`
    /// (or full range for last symbol).
    cdf: Vec<u32>,
    total_count: u32,
    update_cycle: u32,
    updates_until_refresh: u32,
}

impl ArithmeticSymbolModel {
    /// Construct a model with unit initial counts for each symbol.
    ///
    /// The initial state matches the LASzip C++ `ArithmeticModel::init()` contract:
    /// `total_count` starts at zero so the first `refresh()` produces `total_count = symbols`,
    /// and `update_cycle` is reset to `(symbols + 6) >> 1` after that first refresh, exactly
    /// as the C++ code does post-`update()` inside `init()`.
    pub fn new(symbols: u32) -> Self {
        assert!((2..=(1 << 11)).contains(&symbols), "invalid symbol count");

        let counts = vec![1u32; symbols as usize];
        let cdf = vec![0u32; symbols as usize];

        let mut model = Self {
            symbols,
            last_symbol: symbols - 1,
            counts,
            cdf,
            // Start at zero so the first refresh() correctly sets total_count = symbols
            // (matching C++ ArithmeticModel::init() → update() which does
            //  total_count += update_cycle where total_count was 0).
            total_count: 0,
            update_cycle: symbols,
            updates_until_refresh: (symbols + 6) >> 1,
        };
        model.refresh();
        // Mirror the C++ post-init override: after the first update(), init() resets
        // update_cycle (and symbols_until_update) to (symbols + 6) >> 1.
        let post_init_cycle = (symbols + 6) >> 1;
        model.update_cycle = post_init_cycle;
        model.updates_until_refresh = post_init_cycle;
        model
    }

    /// Number of modeled symbols.
    pub fn symbols(&self) -> u32 {
        self.symbols
    }

    /// Last symbol index.
    pub fn last_symbol(&self) -> u32 {
        self.last_symbol
    }

    /// Return scaled CDF entry for `symbol`.
    pub fn cdf_at(&self, symbol: u32) -> u32 {
        self.cdf[symbol as usize]
    }

    /// Find symbol containing scaled value `v` (in `[0, 1 << SYMBOL_LENGTH_SHIFT)`).
    pub fn symbol_for_scaled_value(&self, v: u32) -> u32 {
        let mut lo = 0u32;
        let mut hi = self.symbols;

        while lo + 1 < hi {
            let mid = (lo + hi) >> 1;
            if self.cdf[mid as usize] <= v {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// Notify model that `symbol` was decoded/encoded.
    pub fn observe_symbol(&mut self, symbol: u32) {
        self.counts[symbol as usize] += 1;
        self.updates_until_refresh -= 1;
        if self.updates_until_refresh == 0 {
            self.refresh();
        }
    }

    fn refresh(&mut self) {
        self.total_count += self.update_cycle;
        if self.total_count > SYMBOL_MAX_COUNT {
            self.total_count = 0;
            for c in &mut self.counts {
                *c = (*c + 1) >> 1;
                self.total_count += *c;
            }
        }

        let scale = 0x8000_0000u32 / self.total_count;
        let mut running = 0u32;
        for (dst, count) in self.cdf.iter_mut().zip(self.counts.iter()) {
            *dst = (scale * running) >> (31 - SYMBOL_LENGTH_SHIFT);
            running += *count;
        }

        self.update_cycle = (5 * self.update_cycle) >> 2;
        let max_cycle = (self.symbols + 6) << 3;
        if self.update_cycle > max_cycle {
            self.update_cycle = max_cycle;
        }
        self.updates_until_refresh = self.update_cycle;
    }
}

#[cfg(test)]
mod tests {
    use super::{ArithmeticBitModel, ArithmeticSymbolModel};

    #[test]
    fn bit_model_adapts_towards_observed_zeros() {
        let mut model = ArithmeticBitModel::new();
        let p0_before = model.zero_probability();
        for _ in 0..128 {
            model.observe_bit(false);
        }
        assert!(model.zero_probability() > p0_before);
    }

    #[test]
    fn symbol_model_returns_monotonic_cdf() {
        let mut model = ArithmeticSymbolModel::new(32);
        for i in 0..500 {
            model.observe_symbol((i % 7) as u32);
        }

        let mut prev = 0u32;
        for s in 0..model.symbols() {
            let cur = model.cdf_at(s);
            assert!(cur >= prev);
            prev = cur;
        }
    }
}
