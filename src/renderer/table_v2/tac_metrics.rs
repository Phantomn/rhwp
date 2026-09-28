//! Baseline-relative occupancy shared by stored and recomposed inline tables.
//!
//! Hancom observations in #7049/#7150 place 85% of the table box above its
//! baseline and 15% below. Outside margins extend that box; they do not move
//! each sibling onto a different baseline. Units are chosen by the caller.

pub(super) fn ascent(height: f64) -> f64 {
    height * 0.85
}

pub(super) struct TableBand {
    pub baseline: f64,
    pub height: f64,
}

impl TableBand {
    pub fn measure(tables: impl IntoIterator<Item = (f64, f64, f64)>) -> Option<Self> {
        let mut above = f64::NEG_INFINITY;
        let mut below = f64::NEG_INFINITY;
        let mut any = false;
        for (height, top, bottom) in tables {
            let a = ascent(height);
            above = above.max(a + top);
            below = below.max(height - a + bottom);
            any = true;
        }
        any.then_some(Self {
            baseline: above,
            height: above + below,
        })
    }

    pub fn top(&self, height: f64) -> f64 {
        self.baseline - ascent(height)
    }
}
