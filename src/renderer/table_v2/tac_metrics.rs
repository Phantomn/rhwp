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
    /// A control-only picture/shape line still has a character-height strut.
    /// Extra height is distributed around the baseline, not added below the
    /// object or painted as a second blank line. Normal saved small-line shapes
    /// with asymmetric margins establish alignment of their outer envelope.
    pub fn include_character_height(&mut self, height: f64) {
        if height > self.height {
            self.baseline += ascent(height - self.height);
            self.height = height;
        }
    }
    /// CENTER aligns the complete outer boxes, not the bare table centers.
    /// Independently saved unequal TACs with asymmetric margins establish this
    /// distinction (issue7353_center_tac_review). `baseline` is the center axis
    /// in this mode; it is not an 85% text/table baseline.
    pub fn measure_centered(tables: impl IntoIterator<Item = (f64, f64, f64)>) -> Option<Self> {
        tables
            .into_iter()
            .map(|(height, top, bottom)| height + top + bottom)
            .reduce(f64::max)
            .map(|height| Self {
                baseline: height / 2.0,
                height,
            })
    }

    pub fn centered_top(&self, height: f64, top: f64, bottom: f64) -> f64 {
        self.baseline - (height + top + bottom) / 2.0 + top
    }

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
